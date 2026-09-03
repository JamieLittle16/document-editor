#![doc = "Supervised lifecycle host for trusted bundled product features."]

use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;

use extension_api::{FeatureId, FeatureManifest, FeatureOrigin, ServiceId};
use extension_runtime::{
    FeatureRegistry, FeatureSelection, RegistrationError, ResolveError, ResolvedFeatures,
};

/// Runtime error reported by one bundled feature implementation.
///
/// The host keeps lifecycle failure categories typed while allowing each feature to
/// attach implementation-specific diagnostic text without exposing private error types.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FeatureError {
    message: String,
}

impl FeatureError {
    /// Creates a feature-local runtime error.
    #[must_use]
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    /// Human-readable implementation detail suitable for diagnostics.
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for FeatureError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.message.fmt(formatter)
    }
}

impl Error for FeatureError {}

/// Read-only activation context exposed to bundled feature implementations.
///
/// It deliberately exposes the validated product plan rather than mutable host state.
pub struct FeatureActivationContext<'a> {
    plan: &'a ResolvedFeatures,
}

impl<'a> FeatureActivationContext<'a> {
    const fn new(plan: &'a ResolvedFeatures) -> Self {
        Self { plan }
    }

    /// Returns whether a feature is enabled in the resolved product profile.
    #[must_use]
    pub fn is_enabled(&self, feature: &FeatureId) -> bool {
        self.plan.is_enabled(feature)
    }

    /// Returns the selected provider for a replaceable service, when one is bound.
    #[must_use]
    pub fn provider_for(&self, service: &ServiceId) -> Option<&FeatureId> {
        self.plan.provider(service)
    }
}

/// Trusted, statically linked feature implementation supervised by [`FeatureHost`].
///
/// External extensions must not implement this path directly; they will be hosted behind
/// a sandbox boundary. `deactivate` must be safe after a partially failed `activate`,
/// because the host invokes it while rolling back failed startup.
pub trait BundledFeature: Send {
    /// Stable ID matching the registered [`FeatureManifest`].
    fn id(&self) -> &FeatureId;

    /// Acquires registrations/resources needed by this feature.
    fn activate(&mut self, context: &FeatureActivationContext<'_>) -> Result<(), FeatureError>;

    /// Releases all registrations/resources owned by this feature.
    fn deactivate(&mut self) -> Result<(), FeatureError>;
}

/// Coarse lifecycle state of the bundled feature host.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum HostState {
    /// Catalogue may be extended and a profile may be started.
    #[default]
    Ready,
    /// A validated profile is fully active.
    Running,
    /// Cleanup failed and some feature resources may still be active.
    Faulted,
}

/// Owns the bundled feature catalogue, instances and supervised lifecycle.
///
/// Resolution always completes before the first feature activates. Startup failure
/// triggers reverse cleanup, including cleanup of the feature whose activation failed.
#[derive(Default)]
pub struct FeatureHost {
    registry: FeatureRegistry,
    features: BTreeMap<FeatureId, Box<dyn BundledFeature>>,
    active: Vec<FeatureId>,
    plan: Option<ResolvedFeatures>,
    state: HostState,
}

impl FeatureHost {
    /// Creates an empty host in [`HostState::Ready`].
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Current lifecycle state.
    #[must_use]
    pub const fn state(&self) -> HostState {
        self.state
    }

    /// Active features in activation order.
    #[must_use]
    pub fn active_features(&self) -> &[FeatureId] {
        &self.active
    }

    /// Current validated plan while running or faulted.
    #[must_use]
    pub fn resolved_plan(&self) -> Option<&ResolvedFeatures> {
        self.plan.as_ref()
    }

    /// Registers one trusted bundled feature and its declarative manifest.
    ///
    /// Registration is only permitted while ready. External manifests are rejected so
    /// a future third-party extension cannot accidentally bypass the sandbox host.
    pub fn register(
        &mut self,
        manifest: FeatureManifest,
        feature: Box<dyn BundledFeature>,
    ) -> Result<(), HostRegistrationError> {
        if self.state != HostState::Ready {
            return Err(HostRegistrationError::InvalidState(self.state));
        }

        let manifest_id = manifest.id().clone();
        let implementation_id = feature.id().clone();

        if manifest.origin() != FeatureOrigin::Bundled {
            return Err(HostRegistrationError::ExternalFeature(manifest_id));
        }
        if manifest_id != implementation_id {
            return Err(HostRegistrationError::IdMismatch {
                manifest: manifest_id,
                implementation: implementation_id,
            });
        }
        if self.features.contains_key(&manifest_id) {
            return Err(HostRegistrationError::DuplicateImplementation(manifest_id));
        }

        self.registry
            .register(manifest)
            .map_err(HostRegistrationError::Catalogue)?;
        self.features.insert(manifest_id, feature);
        Ok(())
    }

    /// Resolves and atomically activates one product profile.
    ///
    /// "Atomically" here means no invalid graph starts, and a runtime activation failure
    /// triggers reverse cleanup. If cleanup itself fails, the host enters `Faulted` and
    /// reports every cleanup failure rather than pretending startup rolled back cleanly.
    pub fn start(&mut self, selection: &FeatureSelection) -> Result<(), HostStartError> {
        if self.state != HostState::Ready {
            return Err(HostStartError::InvalidState(self.state));
        }

        let plan = self
            .registry
            .resolve(selection)
            .map_err(HostStartError::Resolution)?;
        let activation_order = plan.activation_order().to_vec();

        for feature in &activation_order {
            if !self.features.contains_key(feature) {
                return Err(HostStartError::MissingImplementation(feature.clone()));
            }
        }

        self.plan = Some(plan.clone());
        let context = FeatureActivationContext::new(&plan);

        for feature_id in activation_order {
            let activation = self
                .features
                .get_mut(&feature_id)
                .expect("resolved feature implementations were validated before activation")
                .activate(&context);

            if let Err(error) = activation {
                let failing_cleanup = self
                    .features
                    .get_mut(&feature_id)
                    .expect("failing feature implementation remains registered")
                    .deactivate();

                let mut rollback_failures = self.deactivate_all_active();
                if let Err(cleanup_error) = failing_cleanup {
                    self.active.push(feature_id.clone());
                    rollback_failures.insert(
                        0,
                        FeatureFailure {
                            feature: feature_id.clone(),
                            error: cleanup_error,
                        },
                    );
                }
                self.finish_cleanup_state();

                return Err(HostStartError::Activation {
                    feature: feature_id,
                    error,
                    rollback_failures,
                });
            }

            self.active.push(feature_id);
        }

        self.state = HostState::Running;
        Ok(())
    }

    /// Deactivates all active features in reverse activation order.
    ///
    /// Calling `stop` while already ready is intentionally idempotent. A faulted host may
    /// be stopped again to retry cleanup of resources whose earlier deactivation failed.
    pub fn stop(&mut self) -> Result<(), HostStopError> {
        if self.state == HostState::Ready {
            return Ok(());
        }

        let failures = self.deactivate_all_active();
        self.finish_cleanup_state();

        if failures.is_empty() {
            Ok(())
        } else {
            Err(HostStopError { failures })
        }
    }

    fn deactivate_all_active(&mut self) -> Vec<FeatureFailure> {
        let active = self.active.iter().rev().cloned().collect::<Vec<_>>();
        let mut failures = Vec::new();
        let mut failed = Vec::new();

        for feature_id in active {
            let result = self
                .features
                .get_mut(&feature_id)
                .expect("active feature implementations cannot disappear")
                .deactivate();
            if let Err(error) = result {
                failed.push(feature_id.clone());
                failures.push(FeatureFailure {
                    feature: feature_id,
                    error,
                });
            }
        }

        failed.reverse();
        self.active = failed;
        failures
    }

    fn finish_cleanup_state(&mut self) {
        if self.active.is_empty() {
            self.state = HostState::Ready;
            self.plan = None;
        } else {
            self.state = HostState::Faulted;
        }
    }
}

/// Registration errors detected before product-profile resolution.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HostRegistrationError {
    /// Runtime catalogue cannot change while active/faulted.
    InvalidState(HostState),
    /// External code must use the future sandbox host, never the bundled in-process path.
    ExternalFeature(FeatureId),
    /// Manifest identity and implementation identity disagree.
    IdMismatch {
        manifest: FeatureId,
        implementation: FeatureId,
    },
    /// Two in-process implementations claimed the same stable feature ID.
    DuplicateImplementation(FeatureId),
    /// Declarative catalogue rejected the manifest.
    Catalogue(RegistrationError),
}

impl fmt::Display for HostRegistrationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidState(state) => {
                write!(
                    formatter,
                    "cannot register bundled feature while host is {state:?}"
                )
            }
            Self::ExternalFeature(feature) => write!(
                formatter,
                "external feature {feature} cannot run through the trusted bundled feature host"
            ),
            Self::IdMismatch {
                manifest,
                implementation,
            } => write!(
                formatter,
                "feature manifest id {manifest} does not match implementation id {implementation}"
            ),
            Self::DuplicateImplementation(feature) => {
                write!(
                    formatter,
                    "feature implementation {feature} is already registered"
                )
            }
            Self::Catalogue(error) => error.fmt(formatter),
        }
    }
}

impl Error for HostRegistrationError {}

/// One failed feature cleanup operation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FeatureFailure {
    /// Feature whose cleanup failed.
    pub feature: FeatureId,
    /// Feature-local diagnostic.
    pub error: FeatureError,
}

/// Startup errors. Resolution errors occur before any activation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HostStartError {
    /// Host must be ready before a new profile starts.
    InvalidState(HostState),
    /// Declarative profile failed resolution before activation.
    Resolution(ResolveError),
    /// Internal catalogue/implementation invariant was violated.
    MissingImplementation(FeatureId),
    /// One feature failed activation; rollback failures are reported separately.
    Activation {
        feature: FeatureId,
        error: FeatureError,
        rollback_failures: Vec<FeatureFailure>,
    },
}

impl fmt::Display for HostStartError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidState(state) => {
                write!(
                    formatter,
                    "cannot start bundled features while host is {state:?}"
                )
            }
            Self::Resolution(error) => {
                write!(formatter, "feature profile resolution failed: {error}")
            }
            Self::MissingImplementation(feature) => write!(
                formatter,
                "resolved feature {feature} has no registered bundled implementation"
            ),
            Self::Activation {
                feature,
                error,
                rollback_failures,
            } => write!(
                formatter,
                "feature {feature} failed activation: {error}; {} rollback cleanup failure(s)",
                rollback_failures.len()
            ),
        }
    }
}

impl Error for HostStartError {}

/// Shutdown error containing every feature that could not clean up.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HostStopError {
    failures: Vec<FeatureFailure>,
}

impl HostStopError {
    /// Cleanup failures collected while continuing best-effort reverse shutdown.
    #[must_use]
    pub fn failures(&self) -> &[FeatureFailure] {
        &self.failures
    }
}

impl fmt::Display for HostStopError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{} bundled feature(s) failed to deactivate",
            self.failures.len()
        )
    }
}

impl Error for HostStopError {}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use extension_api::{FeatureId, FeatureManifest, FeatureOrigin, ServiceId};
    use extension_runtime::FeatureSelection;

    use super::{
        BundledFeature, FeatureActivationContext, FeatureError, FeatureHost, HostRegistrationError,
        HostStartError, HostState,
    };

    fn feature_id(value: &str) -> FeatureId {
        FeatureId::new(value).expect("test feature id is valid")
    }

    fn service_id(value: &str) -> ServiceId {
        ServiceId::new(value).expect("test service id is valid")
    }

    struct RecordingFeature {
        id: FeatureId,
        events: Arc<Mutex<Vec<String>>>,
        fail_activate: bool,
        fail_deactivate: bool,
        observe_service: Option<ServiceId>,
    }

    impl RecordingFeature {
        fn new(id: &str, events: Arc<Mutex<Vec<String>>>) -> Self {
            Self {
                id: feature_id(id),
                events,
                fail_activate: false,
                fail_deactivate: false,
                observe_service: None,
            }
        }

        fn fail_activate(mut self) -> Self {
            self.fail_activate = true;
            self
        }

        fn fail_deactivate(mut self) -> Self {
            self.fail_deactivate = true;
            self
        }

        fn observe(mut self, service: ServiceId) -> Self {
            self.observe_service = Some(service);
            self
        }

        fn push(&self, event: String) {
            self.events.lock().expect("test log mutex").push(event);
        }
    }

    impl BundledFeature for RecordingFeature {
        fn id(&self) -> &FeatureId {
            &self.id
        }

        fn activate(&mut self, context: &FeatureActivationContext<'_>) -> Result<(), FeatureError> {
            self.push(format!("activate:{}", self.id));
            if let Some(service) = &self.observe_service {
                let provider = context
                    .provider_for(service)
                    .map_or("none", FeatureId::as_str);
                self.push(format!("provider:{service}={provider}"));
            }
            if self.fail_activate {
                Err(FeatureError::new("activation failed"))
            } else {
                Ok(())
            }
        }

        fn deactivate(&mut self) -> Result<(), FeatureError> {
            self.push(format!("deactivate:{}", self.id));
            if self.fail_deactivate {
                Err(FeatureError::new("deactivation failed"))
            } else {
                Ok(())
            }
        }
    }

    fn bundled(id: &str) -> FeatureManifest {
        FeatureManifest::new(feature_id(id), FeatureOrigin::Bundled)
    }

    fn register(host: &mut FeatureHost, manifest: FeatureManifest, feature: RecordingFeature) {
        host.register(manifest, Box::new(feature))
            .expect("test feature registers");
    }

    #[test]
    fn starts_in_resolved_order_and_stops_in_reverse_order() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let mut host = FeatureHost::new();

        register(
            &mut host,
            bundled("platform.commands").default_enabled(false),
            RecordingFeature::new("platform.commands", Arc::clone(&events)),
        );
        register(
            &mut host,
            bundled("document.command-palette")
                .default_enabled(true)
                .depends_on(feature_id("platform.commands")),
            RecordingFeature::new("document.command-palette", Arc::clone(&events)),
        );

        host.start(&FeatureSelection::new())
            .expect("valid profile starts");
        assert_eq!(host.state(), HostState::Running);
        assert_eq!(
            host.active_features(),
            [
                feature_id("platform.commands"),
                feature_id("document.command-palette")
            ]
        );

        host.stop().expect("clean stop");
        assert_eq!(host.state(), HostState::Ready);
        assert_eq!(
            *events.lock().expect("test log mutex"),
            [
                "activate:platform.commands",
                "activate:document.command-palette",
                "deactivate:document.command-palette",
                "deactivate:platform.commands"
            ]
        );
    }

    #[test]
    fn resolution_failure_activates_nothing() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let mut host = FeatureHost::new();
        let service = service_id("language.spellcheck");

        register(
            &mut host,
            bundled("language.proofreading")
                .default_enabled(true)
                .requires(service),
            RecordingFeature::new("language.proofreading", Arc::clone(&events)),
        );

        let result = host.start(&FeatureSelection::new());
        assert!(matches!(result, Err(HostStartError::Resolution(_))));
        assert!(events.lock().expect("test log mutex").is_empty());
        assert_eq!(host.state(), HostState::Ready);
    }

    #[test]
    fn activation_failure_cleans_failing_feature_and_rolls_back_prior_features() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let mut host = FeatureHost::new();

        register(
            &mut host,
            bundled("feature.a").default_enabled(true),
            RecordingFeature::new("feature.a", Arc::clone(&events)),
        );
        register(
            &mut host,
            bundled("feature.b")
                .default_enabled(true)
                .depends_on(feature_id("feature.a")),
            RecordingFeature::new("feature.b", Arc::clone(&events)).fail_activate(),
        );

        let result = host.start(&FeatureSelection::new());
        assert!(matches!(
            result,
            Err(HostStartError::Activation {
                rollback_failures,
                ..
            }) if rollback_failures.is_empty()
        ));
        assert_eq!(host.state(), HostState::Ready);
        assert!(host.active_features().is_empty());
        assert_eq!(
            *events.lock().expect("test log mutex"),
            [
                "activate:feature.a",
                "activate:feature.b",
                "deactivate:feature.b",
                "deactivate:feature.a"
            ]
        );
    }

    #[test]
    fn rollback_failure_faults_host_and_preserves_cleanup_retry() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let mut host = FeatureHost::new();

        register(
            &mut host,
            bundled("feature.a").default_enabled(true),
            RecordingFeature::new("feature.a", Arc::clone(&events)).fail_deactivate(),
        );
        register(
            &mut host,
            bundled("feature.b")
                .default_enabled(true)
                .depends_on(feature_id("feature.a")),
            RecordingFeature::new("feature.b", Arc::clone(&events)).fail_activate(),
        );

        let result = host.start(&FeatureSelection::new());
        assert!(matches!(
            result,
            Err(HostStartError::Activation {
                rollback_failures,
                ..
            }) if rollback_failures.len() == 1
        ));
        assert_eq!(host.state(), HostState::Faulted);
        assert_eq!(host.active_features(), [feature_id("feature.a")]);

        let stop = host.stop().expect_err("feature a still fails cleanup");
        assert_eq!(stop.failures().len(), 1);
        assert_eq!(host.state(), HostState::Faulted);
    }

    #[test]
    fn failing_activator_cleanup_failure_remains_tracked() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let mut host = FeatureHost::new();

        register(
            &mut host,
            bundled("feature.a").default_enabled(true),
            RecordingFeature::new("feature.a", Arc::clone(&events)),
        );
        register(
            &mut host,
            bundled("feature.b")
                .default_enabled(true)
                .depends_on(feature_id("feature.a")),
            RecordingFeature::new("feature.b", Arc::clone(&events))
                .fail_activate()
                .fail_deactivate(),
        );

        let result = host.start(&FeatureSelection::new());
        assert!(matches!(
            result,
            Err(HostStartError::Activation {
                rollback_failures,
                ..
            }) if rollback_failures.len() == 1
        ));
        assert_eq!(host.state(), HostState::Faulted);
        assert_eq!(host.active_features(), [feature_id("feature.b")]);
        assert_eq!(
            *events.lock().expect("test log mutex"),
            [
                "activate:feature.a",
                "activate:feature.b",
                "deactivate:feature.b",
                "deactivate:feature.a"
            ]
        );
    }

    #[test]
    fn selected_provider_is_activated_before_consumer_and_visible_in_context() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let mut host = FeatureHost::new();
        let spellcheck = service_id("language.spellcheck");

        register(
            &mut host,
            bundled("language.basic-spellcheck").provides(spellcheck.clone()),
            RecordingFeature::new("language.basic-spellcheck", Arc::clone(&events)),
        );
        register(
            &mut host,
            bundled("language.advanced-spellcheck").provides(spellcheck.clone()),
            RecordingFeature::new("language.advanced-spellcheck", Arc::clone(&events)),
        );
        register(
            &mut host,
            bundled("language.proofreading")
                .default_enabled(true)
                .requires(spellcheck.clone()),
            RecordingFeature::new("language.proofreading", Arc::clone(&events))
                .observe(spellcheck.clone()),
        );

        let mut selection = FeatureSelection::new();
        selection.prefer_provider(spellcheck, feature_id("language.advanced-spellcheck"));

        host.start(&selection).expect("provider profile starts");
        assert_eq!(
            *events.lock().expect("test log mutex"),
            [
                "activate:language.advanced-spellcheck",
                "activate:language.proofreading",
                "provider:language.spellcheck=language.advanced-spellcheck"
            ]
        );
    }

    #[test]
    fn external_feature_cannot_use_trusted_in_process_host() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let mut host = FeatureHost::new();
        let id = feature_id("external.example");
        let manifest = FeatureManifest::new(id.clone(), FeatureOrigin::External);

        let result = host.register(
            manifest,
            Box::new(RecordingFeature::new("external.example", events)),
        );

        assert_eq!(result, Err(HostRegistrationError::ExternalFeature(id)));
    }

    #[test]
    fn manifest_and_implementation_ids_must_match() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let mut host = FeatureHost::new();

        let result = host.register(
            bundled("feature.manifest"),
            Box::new(RecordingFeature::new("feature.implementation", events)),
        );

        assert_eq!(
            result,
            Err(HostRegistrationError::IdMismatch {
                manifest: feature_id("feature.manifest"),
                implementation: feature_id("feature.implementation")
            })
        );
    }
}
