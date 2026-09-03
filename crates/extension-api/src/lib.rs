#![doc = "Stable product-level contracts for bundled features and external extensions."]

use std::collections::BTreeSet;
use std::error::Error;
use std::fmt;

const MAX_IDENTIFIER_LEN: usize = 128;

/// Stable identifier for a product feature or extension.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FeatureId(String);

impl FeatureId {
    /// Creates a validated feature identifier.
    ///
    /// Identifiers are deliberately boring and stable because they may appear in
    /// persisted configuration, extension manifests, diagnostics and crash logs.
    pub fn new(value: impl Into<String>) -> Result<Self, IdentifierError> {
        let value = value.into();
        validate_identifier(&value)?;
        Ok(Self(value))
    }

    /// Returns the identifier as text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for FeatureId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

impl TryFrom<&str> for FeatureId {
    type Error = IdentifierError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

/// Stable identifier for a replaceable product service.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ServiceId(String);

impl ServiceId {
    /// Creates a validated service identifier.
    pub fn new(value: impl Into<String>) -> Result<Self, IdentifierError> {
        let value = value.into();
        validate_identifier(&value)?;
        Ok(Self(value))
    }

    /// Returns the identifier as text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ServiceId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

impl TryFrom<&str> for ServiceId {
    type Error = IdentifierError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

/// Where a feature implementation comes from.
///
/// Bundled and external features share product contracts, but not necessarily a
/// trust boundary or loading mechanism.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FeatureOrigin {
    /// Shipped and versioned with the application.
    Bundled,
    /// Supplied separately and expected to run behind a sandbox boundary.
    External,
}

/// Declarative description of a feature's composition requirements.
///
/// The manifest intentionally contains no engine implementation types and no UI
/// framework types. It is product architecture, not adapter architecture.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FeatureManifest {
    id: FeatureId,
    origin: FeatureOrigin,
    default_enabled: bool,
    dependencies: BTreeSet<FeatureId>,
    conflicts: BTreeSet<FeatureId>,
    provides: BTreeSet<ServiceId>,
    requires: BTreeSet<ServiceId>,
}

impl FeatureManifest {
    /// Creates a feature manifest with conservative defaults.
    #[must_use]
    pub fn new(id: FeatureId, origin: FeatureOrigin) -> Self {
        Self {
            id,
            origin,
            default_enabled: false,
            dependencies: BTreeSet::new(),
            conflicts: BTreeSet::new(),
            provides: BTreeSet::new(),
            requires: BTreeSet::new(),
        }
    }

    /// Marks whether the feature is enabled in the default product profile.
    #[must_use]
    pub fn default_enabled(mut self, enabled: bool) -> Self {
        self.default_enabled = enabled;
        self
    }

    /// Declares a hard feature dependency.
    #[must_use]
    pub fn depends_on(mut self, dependency: FeatureId) -> Self {
        self.dependencies.insert(dependency);
        self
    }

    /// Declares that this feature cannot be active with another feature.
    #[must_use]
    pub fn conflicts_with(mut self, conflict: FeatureId) -> Self {
        self.conflicts.insert(conflict);
        self
    }

    /// Declares a replaceable service provided by this feature.
    #[must_use]
    pub fn provides(mut self, service: ServiceId) -> Self {
        self.provides.insert(service);
        self
    }

    /// Declares a service required before this feature may activate.
    #[must_use]
    pub fn requires(mut self, service: ServiceId) -> Self {
        self.requires.insert(service);
        self
    }

    /// Stable feature identifier.
    #[must_use]
    pub fn id(&self) -> &FeatureId {
        &self.id
    }

    /// Feature origin.
    #[must_use]
    pub const fn origin(&self) -> FeatureOrigin {
        self.origin
    }

    /// Whether the default profile enables this feature.
    #[must_use]
    pub const fn is_default_enabled(&self) -> bool {
        self.default_enabled
    }

    /// Hard feature dependencies.
    pub fn dependencies(&self) -> impl Iterator<Item = &FeatureId> {
        self.dependencies.iter()
    }

    /// Mutually incompatible features.
    pub fn conflicts(&self) -> impl Iterator<Item = &FeatureId> {
        self.conflicts.iter()
    }

    /// Replaceable services supplied by this feature.
    pub fn provided_services(&self) -> impl Iterator<Item = &ServiceId> {
        self.provides.iter()
    }

    /// Replaceable services consumed by this feature.
    pub fn required_services(&self) -> impl Iterator<Item = &ServiceId> {
        self.requires.iter()
    }

    /// Returns whether this feature provides a service.
    #[must_use]
    pub fn provides_service(&self, service: &ServiceId) -> bool {
        self.provides.contains(service)
    }
}

/// Error returned for unstable or malformed public identifiers.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IdentifierError {
    value: String,
}

impl IdentifierError {
    /// The rejected identifier.
    #[must_use]
    pub fn value(&self) -> &str {
        &self.value
    }
}

impl fmt::Display for IdentifierError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "invalid identifier {:?}: use 1-{MAX_IDENTIFIER_LEN} lowercase ASCII letters, digits, '.', '-' or '_' and start with a letter or digit",
            self.value
        )
    }
}

impl Error for IdentifierError {}

fn validate_identifier(value: &str) -> Result<(), IdentifierError> {
    let valid_length = !value.is_empty() && value.len() <= MAX_IDENTIFIER_LEN;
    let mut characters = value.chars();
    let valid_start = characters
        .next()
        .is_some_and(|character| character.is_ascii_lowercase() || character.is_ascii_digit());
    let valid_rest = characters.all(|character| {
        character.is_ascii_lowercase()
            || character.is_ascii_digit()
            || matches!(character, '.' | '-' | '_')
    });

    if valid_length && valid_start && valid_rest {
        Ok(())
    } else {
        Err(IdentifierError {
            value: value.to_owned(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{FeatureId, FeatureManifest, FeatureOrigin, ServiceId};

    #[test]
    fn identifiers_are_stable_and_deliberately_restricted() {
        assert!(FeatureId::new("document.history.timeline").is_ok());
        assert!(ServiceId::new("language.spellcheck").is_ok());
        assert!(FeatureId::new("Uppercase").is_err());
        assert!(FeatureId::new("contains space").is_err());
        assert!(FeatureId::new("").is_err());
    }

    #[test]
    fn manifest_deduplicates_declarative_edges() {
        let dependency = FeatureId::new("document.commands").expect("valid id");
        let service = ServiceId::new("language.spellcheck").expect("valid id");
        let manifest = FeatureManifest::new(
            FeatureId::new("language.ui").expect("valid id"),
            FeatureOrigin::Bundled,
        )
        .depends_on(dependency.clone())
        .depends_on(dependency)
        .requires(service.clone())
        .requires(service);

        assert_eq!(manifest.dependencies().count(), 1);
        assert_eq!(manifest.required_services().count(), 1);
    }
}
