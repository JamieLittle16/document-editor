#![doc = "Deterministic feature composition and replaceable-service resolution."]

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;

use extension_api::{FeatureId, FeatureManifest, ServiceId};

/// Registry of all feature manifests known to the current application build.
#[derive(Clone, Debug, Default)]
pub struct FeatureRegistry {
    manifests: BTreeMap<FeatureId, FeatureManifest>,
}

impl FeatureRegistry {
    /// Creates an empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers one feature manifest.
    pub fn register(&mut self, manifest: FeatureManifest) -> Result<(), RegistrationError> {
        let id = manifest.id().clone();
        if self.manifests.contains_key(&id) {
            return Err(RegistrationError::DuplicateFeature(id));
        }
        self.manifests.insert(id, manifest);
        Ok(())
    }

    /// Returns a registered feature manifest.
    #[must_use]
    pub fn manifest(&self, id: &FeatureId) -> Option<&FeatureManifest> {
        self.manifests.get(id)
    }

    /// Resolves the active feature set, provider bindings and deterministic activation order.
    pub fn resolve(&self, selection: &FeatureSelection) -> Result<ResolvedFeatures, ResolveError> {
        self.validate_selection(selection)?;

        let mut enabled = self
            .manifests
            .values()
            .filter(|manifest| manifest.is_default_enabled())
            .map(|manifest| manifest.id().clone())
            .collect::<BTreeSet<_>>();
        enabled.extend(selection.enabled.iter().cloned());
        for id in &selection.disabled {
            enabled.remove(id);
        }

        for (service, provider) in &selection.preferred_providers {
            if selection.disabled.contains(provider) {
                return Err(ResolveError::PreferredProviderDisabled {
                    service: service.clone(),
                    provider: provider.clone(),
                });
            }
            let manifest = self
                .manifests
                .get(provider)
                .expect("selection validation guarantees provider registration");
            if !manifest.provides_service(service) {
                return Err(ResolveError::PreferredProviderDoesNotProvide {
                    service: service.clone(),
                    provider: provider.clone(),
                });
            }
            enabled.insert(provider.clone());
        }

        self.close_dependencies(&mut enabled, selection)?;
        self.validate_conflicts(&enabled)?;
        let providers = self.resolve_services(&enabled, selection)?;
        let activation_order = self.activation_order(&enabled, &providers)?;

        Ok(ResolvedFeatures {
            enabled,
            providers,
            activation_order,
        })
    }

    fn validate_selection(&self, selection: &FeatureSelection) -> Result<(), ResolveError> {
        for id in selection.enabled.iter().chain(&selection.disabled) {
            if !self.manifests.contains_key(id) {
                return Err(ResolveError::UnknownFeature(id.clone()));
            }
        }
        for provider in selection.preferred_providers.values() {
            if !self.manifests.contains_key(provider) {
                return Err(ResolveError::UnknownFeature(provider.clone()));
            }
        }
        Ok(())
    }

    fn close_dependencies(
        &self,
        enabled: &mut BTreeSet<FeatureId>,
        selection: &FeatureSelection,
    ) -> Result<(), ResolveError> {
        let mut pending = enabled.iter().cloned().collect::<Vec<_>>();

        while let Some(id) = pending.pop() {
            let manifest = self
                .manifests
                .get(&id)
                .expect("enabled features are registered");
            for dependency in manifest.dependencies() {
                if !self.manifests.contains_key(dependency) {
                    return Err(ResolveError::MissingDependency {
                        feature: id.clone(),
                        dependency: dependency.clone(),
                    });
                }
                if selection.disabled.contains(dependency) {
                    return Err(ResolveError::DependencyDisabled {
                        feature: id.clone(),
                        dependency: dependency.clone(),
                    });
                }
                if enabled.insert(dependency.clone()) {
                    pending.push(dependency.clone());
                }
            }
        }
        Ok(())
    }

    fn validate_conflicts(&self, enabled: &BTreeSet<FeatureId>) -> Result<(), ResolveError> {
        for id in enabled {
            let manifest = self
                .manifests
                .get(id)
                .expect("enabled features are registered");
            for conflict in manifest.conflicts() {
                if !self.manifests.contains_key(conflict) {
                    return Err(ResolveError::UnknownConflictTarget {
                        feature: id.clone(),
                        conflict: conflict.clone(),
                    });
                }
                if enabled.contains(conflict) {
                    return Err(ResolveError::Conflict {
                        first: id.clone(),
                        second: conflict.clone(),
                    });
                }
            }
        }
        Ok(())
    }

    fn resolve_services(
        &self,
        enabled: &BTreeSet<FeatureId>,
        selection: &FeatureSelection,
    ) -> Result<BTreeMap<ServiceId, FeatureId>, ResolveError> {
        let mut providers = selection.preferred_providers.clone();

        for feature in enabled {
            let manifest = self
                .manifests
                .get(feature)
                .expect("enabled features are registered");
            for service in manifest.required_services() {
                if providers.contains_key(service) {
                    continue;
                }

                let candidates = enabled
                    .iter()
                    .filter(|candidate| {
                        self.manifests
                            .get(*candidate)
                            .is_some_and(|candidate_manifest| {
                                candidate_manifest.provides_service(service)
                            })
                    })
                    .cloned()
                    .collect::<Vec<_>>();

                match candidates.as_slice() {
                    [] => {
                        return Err(ResolveError::MissingService {
                            feature: feature.clone(),
                            service: service.clone(),
                        });
                    }
                    [provider] => {
                        providers.insert(service.clone(), provider.clone());
                    }
                    _ => {
                        return Err(ResolveError::AmbiguousService {
                            service: service.clone(),
                            providers: candidates,
                        });
                    }
                }
            }
        }

        Ok(providers)
    }

    fn activation_order(
        &self,
        enabled: &BTreeSet<FeatureId>,
        providers: &BTreeMap<ServiceId, FeatureId>,
    ) -> Result<Vec<FeatureId>, ResolveError> {
        let mut marks = BTreeMap::<FeatureId, VisitState>::new();
        let mut stack = Vec::<FeatureId>::new();
        let mut order = Vec::<FeatureId>::with_capacity(enabled.len());

        for id in enabled {
            self.visit(id, enabled, providers, &mut marks, &mut stack, &mut order)?;
        }

        Ok(order)
    }

    fn visit(
        &self,
        id: &FeatureId,
        enabled: &BTreeSet<FeatureId>,
        providers: &BTreeMap<ServiceId, FeatureId>,
        marks: &mut BTreeMap<FeatureId, VisitState>,
        stack: &mut Vec<FeatureId>,
        order: &mut Vec<FeatureId>,
    ) -> Result<(), ResolveError> {
        match marks.get(id) {
            Some(VisitState::Visited) => return Ok(()),
            Some(VisitState::Visiting) => {
                let start = stack.iter().position(|entry| entry == id).unwrap_or(0);
                let mut cycle = stack[start..].to_vec();
                cycle.push(id.clone());
                return Err(ResolveError::DependencyCycle(cycle));
            }
            None => {}
        }

        marks.insert(id.clone(), VisitState::Visiting);
        stack.push(id.clone());

        let manifest = self
            .manifests
            .get(id)
            .expect("enabled features are registered");
        let mut prerequisites = manifest
            .dependencies()
            .filter(|dependency| enabled.contains(*dependency))
            .cloned()
            .collect::<BTreeSet<_>>();
        for service in manifest.required_services() {
            let provider = providers
                .get(service)
                .expect("required services are resolved before ordering");
            if provider != id {
                prerequisites.insert(provider.clone());
            }
        }

        for prerequisite in prerequisites {
            self.visit(&prerequisite, enabled, providers, marks, stack, order)?;
        }

        let popped = stack.pop();
        debug_assert_eq!(popped.as_ref(), Some(id));
        marks.insert(id.clone(), VisitState::Visited);
        order.push(id.clone());
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum VisitState {
    Visiting,
    Visited,
}

/// User/product-profile choices applied to the registered feature graph.
#[derive(Clone, Debug, Default)]
pub struct FeatureSelection {
    enabled: BTreeSet<FeatureId>,
    disabled: BTreeSet<FeatureId>,
    preferred_providers: BTreeMap<ServiceId, FeatureId>,
}

impl FeatureSelection {
    /// Creates an empty selection, meaning "use product defaults".
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Explicitly enables a feature.
    pub fn enable(&mut self, id: FeatureId) {
        self.disabled.remove(&id);
        self.enabled.insert(id);
    }

    /// Explicitly disables a feature.
    pub fn disable(&mut self, id: FeatureId) {
        self.enabled.remove(&id);
        self.disabled.insert(id);
    }

    /// Selects one feature as the provider for a replaceable service slot.
    pub fn prefer_provider(&mut self, service: ServiceId, provider: FeatureId) {
        self.preferred_providers.insert(service, provider);
    }
}

/// Fully validated activation plan for one product profile.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResolvedFeatures {
    enabled: BTreeSet<FeatureId>,
    providers: BTreeMap<ServiceId, FeatureId>,
    activation_order: Vec<FeatureId>,
}

impl ResolvedFeatures {
    /// Returns whether a feature is active.
    #[must_use]
    pub fn is_enabled(&self, id: &FeatureId) -> bool {
        self.enabled.contains(id)
    }

    /// Returns the bound provider for a replaceable service, when one was needed or selected.
    #[must_use]
    pub fn provider(&self, service: &ServiceId) -> Option<&FeatureId> {
        self.providers.get(service)
    }

    /// Deterministic provider/dependency-before-consumer activation order.
    #[must_use]
    pub fn activation_order(&self) -> &[FeatureId] {
        &self.activation_order
    }

    /// Number of active features.
    #[must_use]
    pub fn len(&self) -> usize {
        self.enabled.len()
    }

    /// Returns whether no features are active.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.enabled.is_empty()
    }
}

/// Registration-time errors that indicate a malformed build/catalogue.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RegistrationError {
    /// A stable feature identifier may only be registered once.
    DuplicateFeature(FeatureId),
}

impl fmt::Display for RegistrationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateFeature(id) => write!(formatter, "feature {id} is already registered"),
        }
    }
}

impl Error for RegistrationError {}

/// Composition error returned before any feature activation occurs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ResolveError {
    UnknownFeature(FeatureId),
    MissingDependency {
        feature: FeatureId,
        dependency: FeatureId,
    },
    DependencyDisabled {
        feature: FeatureId,
        dependency: FeatureId,
    },
    UnknownConflictTarget {
        feature: FeatureId,
        conflict: FeatureId,
    },
    Conflict {
        first: FeatureId,
        second: FeatureId,
    },
    PreferredProviderDisabled {
        service: ServiceId,
        provider: FeatureId,
    },
    PreferredProviderDoesNotProvide {
        service: ServiceId,
        provider: FeatureId,
    },
    MissingService {
        feature: FeatureId,
        service: ServiceId,
    },
    AmbiguousService {
        service: ServiceId,
        providers: Vec<FeatureId>,
    },
    DependencyCycle(Vec<FeatureId>),
}

impl fmt::Display for ResolveError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownFeature(id) => write!(formatter, "unknown feature {id}"),
            Self::MissingDependency {
                feature,
                dependency,
            } => write!(
                formatter,
                "feature {feature} depends on unregistered feature {dependency}"
            ),
            Self::DependencyDisabled {
                feature,
                dependency,
            } => write!(
                formatter,
                "feature {feature} requires explicitly disabled feature {dependency}"
            ),
            Self::UnknownConflictTarget { feature, conflict } => write!(
                formatter,
                "feature {feature} conflicts with unregistered feature {conflict}"
            ),
            Self::Conflict { first, second } => {
                write!(
                    formatter,
                    "features {first} and {second} cannot both be active"
                )
            }
            Self::PreferredProviderDisabled { service, provider } => write!(
                formatter,
                "preferred provider {provider} for service {service} is explicitly disabled"
            ),
            Self::PreferredProviderDoesNotProvide { service, provider } => write!(
                formatter,
                "feature {provider} was selected for service {service} but does not provide it"
            ),
            Self::MissingService { feature, service } => {
                write!(
                    formatter,
                    "feature {feature} requires service {service} with no active provider"
                )
            }
            Self::AmbiguousService { service, providers } => write!(
                formatter,
                "service {service} has multiple active providers and no preference: {providers:?}"
            ),
            Self::DependencyCycle(cycle) => {
                write!(
                    formatter,
                    "feature activation contains a dependency cycle: {cycle:?}"
                )
            }
        }
    }
}

impl Error for ResolveError {}

#[cfg(test)]
mod tests {
    use extension_api::{FeatureId, FeatureManifest, FeatureOrigin, ServiceId};

    use super::{FeatureRegistry, FeatureSelection, RegistrationError, ResolveError};

    fn feature_id(value: &str) -> FeatureId {
        FeatureId::new(value).expect("test ids are valid")
    }

    fn service_id(value: &str) -> ServiceId {
        ServiceId::new(value).expect("test ids are valid")
    }

    fn bundled(value: &str) -> FeatureManifest {
        FeatureManifest::new(feature_id(value), FeatureOrigin::Bundled)
    }

    #[test]
    fn defaults_close_over_dependencies_and_activate_dependencies_first() {
        let mut registry = FeatureRegistry::new();
        registry
            .register(bundled("platform.commands").default_enabled(false))
            .expect("register commands");
        registry
            .register(
                bundled("document.command-palette")
                    .default_enabled(true)
                    .depends_on(feature_id("platform.commands")),
            )
            .expect("register palette");

        let resolved = registry
            .resolve(&FeatureSelection::new())
            .expect("valid composition");

        assert!(resolved.is_enabled(&feature_id("platform.commands")));
        assert_eq!(
            resolved.activation_order(),
            [
                feature_id("platform.commands"),
                feature_id("document.command-palette")
            ]
        );
    }

    #[test]
    fn explicit_disable_wins_over_implicit_dependency_enablement() {
        let mut registry = FeatureRegistry::new();
        registry
            .register(bundled("platform.commands"))
            .expect("register commands");
        registry
            .register(
                bundled("document.command-palette")
                    .default_enabled(true)
                    .depends_on(feature_id("platform.commands")),
            )
            .expect("register palette");
        let mut selection = FeatureSelection::new();
        selection.disable(feature_id("platform.commands"));

        assert!(matches!(
            registry.resolve(&selection),
            Err(ResolveError::DependencyDisabled { .. })
        ));
    }

    #[test]
    fn replaceable_service_provider_is_selected_explicitly() {
        let spellcheck = service_id("language.spellcheck");
        let mut registry = FeatureRegistry::new();
        registry
            .register(
                bundled("language.proofreading-ui")
                    .default_enabled(true)
                    .requires(spellcheck.clone()),
            )
            .expect("register consumer");
        registry
            .register(
                bundled("language.spellcheck.basic")
                    .default_enabled(true)
                    .provides(spellcheck.clone()),
            )
            .expect("register basic provider");
        registry
            .register(
                bundled("language.spellcheck.advanced")
                    .default_enabled(true)
                    .provides(spellcheck.clone()),
            )
            .expect("register advanced provider");

        assert!(matches!(
            registry.resolve(&FeatureSelection::new()),
            Err(ResolveError::AmbiguousService { .. })
        ));

        let mut selection = FeatureSelection::new();
        selection.prefer_provider(
            spellcheck.clone(),
            feature_id("language.spellcheck.advanced"),
        );
        let resolved = registry.resolve(&selection).expect("provider is selected");

        assert_eq!(
            resolved.provider(&spellcheck),
            Some(&feature_id("language.spellcheck.advanced"))
        );
        let provider_position = resolved
            .activation_order()
            .iter()
            .position(|id| id == &feature_id("language.spellcheck.advanced"))
            .expect("provider active");
        let consumer_position = resolved
            .activation_order()
            .iter()
            .position(|id| id == &feature_id("language.proofreading-ui"))
            .expect("consumer active");
        assert!(provider_position < consumer_position);
    }

    #[test]
    fn preferred_provider_is_enabled_even_when_not_enabled_by_default() {
        let exporter = service_id("document.export.pdf");
        let mut registry = FeatureRegistry::new();
        registry
            .register(bundled("export.pdf.native").provides(exporter.clone()))
            .expect("register provider");
        let mut selection = FeatureSelection::new();
        selection.prefer_provider(exporter.clone(), feature_id("export.pdf.native"));

        let resolved = registry
            .resolve(&selection)
            .expect("provider preference is valid");
        assert!(resolved.is_enabled(&feature_id("export.pdf.native")));
        assert_eq!(
            resolved.provider(&exporter),
            Some(&feature_id("export.pdf.native"))
        );
    }

    #[test]
    fn conflicts_are_rejected_before_activation() {
        let mut registry = FeatureRegistry::new();
        registry
            .register(
                bundled("ui.classic")
                    .default_enabled(true)
                    .conflicts_with(feature_id("ui.experimental")),
            )
            .expect("register classic");
        registry
            .register(bundled("ui.experimental").default_enabled(true))
            .expect("register experimental");

        assert!(matches!(
            registry.resolve(&FeatureSelection::new()),
            Err(ResolveError::Conflict { .. })
        ));
    }

    #[test]
    fn dependency_cycles_are_reported() {
        let mut registry = FeatureRegistry::new();
        registry
            .register(
                bundled("feature.a")
                    .default_enabled(true)
                    .depends_on(feature_id("feature.b")),
            )
            .expect("register a");
        registry
            .register(
                bundled("feature.b")
                    .default_enabled(true)
                    .depends_on(feature_id("feature.a")),
            )
            .expect("register b");

        assert!(matches!(
            registry.resolve(&FeatureSelection::new()),
            Err(ResolveError::DependencyCycle(_))
        ));
    }

    #[test]
    fn duplicate_feature_ids_are_rejected() {
        let mut registry = FeatureRegistry::new();
        registry
            .register(bundled("document.history-ui"))
            .expect("first registration succeeds");

        assert_eq!(
            registry.register(bundled("document.history-ui")),
            Err(RegistrationError::DuplicateFeature(feature_id(
                "document.history-ui"
            )))
        );
    }

    #[test]
    fn missing_service_is_reported_without_partial_activation() {
        let service = service_id("language.grammar");
        let mut registry = FeatureRegistry::new();
        registry
            .register(
                bundled("language.grammar-ui")
                    .default_enabled(true)
                    .requires(service),
            )
            .expect("register consumer");

        assert!(matches!(
            registry.resolve(&FeatureSelection::new()),
            Err(ResolveError::MissingService { .. })
        ));
    }
}
