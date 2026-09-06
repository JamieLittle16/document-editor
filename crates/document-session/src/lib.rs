#![doc = "Authority- and revision-aware orchestration around a replaceable document engine."]

use std::fmt;

use document_engine_api::{DocumentEngine, EngineError, SemanticObservation};
use document_protocol::{DocumentRevision, DocumentTransaction, TransactionApplied};

/// One application-owned incarnation of the authoritative document binding.
///
/// A generation changes when this session successfully binds to a newly opened/reopened
/// authority. It is deliberately not a durable `DocumentId`: one logical document may survive
/// several authority generations as workers restart or engine bindings are replaced.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct AuthorityGeneration(u64);

impl AuthorityGeneration {
    const BEFORE_FIRST_OPEN: Self = Self(0);

    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }

    fn checked_next(self) -> Option<Self> {
        self.0.checked_add(1).map(Self)
    }
}

impl fmt::Display for AuthorityGeneration {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

/// Semantic data scoped to one exact session authority generation and document revision.
///
/// The underlying engine observation remains engine-neutral and revision-stamped. The session
/// adds the product-owned authority generation after validating the engine/session revision
/// invariant. There is intentionally no public constructor: callers may transform an observation
/// while preserving provenance, but only `DocumentSession` can mint current authority scope.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SessionObservation<T> {
    authority_generation: AuthorityGeneration,
    engine_observation: SemanticObservation<T>,
}

impl<T> SessionObservation<T> {
    fn new(
        authority_generation: AuthorityGeneration,
        engine_observation: SemanticObservation<T>,
    ) -> Self {
        Self {
            authority_generation,
            engine_observation,
        }
    }

    #[must_use]
    pub const fn authority_generation(&self) -> AuthorityGeneration {
        self.authority_generation
    }

    #[must_use]
    pub const fn revision(&self) -> DocumentRevision {
        self.engine_observation.revision()
    }

    #[must_use]
    pub const fn value(&self) -> &T {
        self.engine_observation.value()
    }

    #[must_use]
    pub fn into_value(self) -> T {
        self.engine_observation.into_value()
    }

    #[must_use]
    pub fn map<U>(self, map: impl FnOnce(T) -> U) -> SessionObservation<U> {
        SessionObservation::new(self.authority_generation, self.engine_observation.map(map))
    }
}

/// Why a previously produced session observation cannot be consumed as current state.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ObservationFreshnessError {
    NoOpenDocument,
    AuthorityChanged {
        observed: AuthorityGeneration,
        current: AuthorityGeneration,
    },
    Stale {
        observed: DocumentRevision,
        current: DocumentRevision,
    },
}

impl fmt::Display for ObservationFreshnessError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoOpenDocument => formatter.write_str("no authoritative document is open"),
            Self::AuthorityChanged { observed, current } => write!(
                formatter,
                "semantic observation belongs to authority generation {observed}, current generation is {current}"
            ),
            Self::Stale { observed, current } => write!(
                formatter,
                "semantic observation is stale: observed revision {observed}, current revision {current}"
            ),
        }
    }
}

impl std::error::Error for ObservationFreshnessError {}

pub struct DocumentSession<E> {
    engine: E,
    authority_generation: AuthorityGeneration,
    revision: Option<DocumentRevision>,
}

impl<E: DocumentEngine> DocumentSession<E> {
    #[must_use]
    pub const fn new(engine: E) -> Self {
        Self {
            engine,
            authority_generation: AuthorityGeneration::BEFORE_FIRST_OPEN,
            revision: None,
        }
    }

    pub fn open_text_fixture(&mut self, text: String) -> Result<DocumentRevision, EngineError> {
        // Reserve the next generation before asking the engine to replace authority. This makes
        // exhaustion fail without touching the current engine binding, while a normal engine
        // failure leaves the current generation/revision unchanged.
        let next_authority = self.authority_generation.checked_next().ok_or_else(|| {
            EngineError::Internal(String::from("document authority generation exhausted"))
        })?;
        let revision = self.engine.open_text_fixture(text)?;
        self.authority_generation = next_authority;
        self.revision = Some(revision);
        Ok(revision)
    }

    #[must_use]
    pub const fn known_revision(&self) -> Option<DocumentRevision> {
        self.revision
    }

    #[must_use]
    pub const fn known_authority_generation(&self) -> Option<AuthorityGeneration> {
        if self.revision.is_some() {
            Some(self.authority_generation)
        } else {
            None
        }
    }

    /// Reads semantic text and scopes it to this exact session authority incarnation.
    ///
    /// A successful read is coherent at the instant it is returned. Callers that retain the
    /// observation across work or await points must call [`Self::require_current`] before using
    /// it to affect current UI/application state.
    pub fn semantic_text(&self) -> Result<SessionObservation<String>, EngineError> {
        let observation = self.engine.semantic_text()?;
        let Some(current) = self.revision else {
            return Err(EngineError::Internal(String::from(
                "engine returned semantic state while session had no authoritative revision",
            )));
        };
        if observation.revision() != current {
            return Err(EngineError::Internal(format!(
                "engine semantic revision {} disagrees with session revision {current}",
                observation.revision()
            )));
        }
        Ok(SessionObservation::new(
            self.authority_generation,
            observation,
        ))
    }

    /// Rejects an observation from another authority generation or an older revision.
    ///
    /// This check is intentionally local and allocation-free. It is the primitive that future
    /// asynchronous search, diagnostics, comments and other feature modules use before applying
    /// derived semantic results to current state. Authority is checked before revision so an old
    /// `R0` cannot become current again merely because a replacement authority also starts at
    /// `R0`.
    pub fn require_current<T>(
        &self,
        observation: &SessionObservation<T>,
    ) -> Result<(), ObservationFreshnessError> {
        let current_revision = self
            .revision
            .ok_or(ObservationFreshnessError::NoOpenDocument)?;
        if observation.authority_generation() != self.authority_generation {
            return Err(ObservationFreshnessError::AuthorityChanged {
                observed: observation.authority_generation(),
                current: self.authority_generation,
            });
        }
        if observation.revision() != current_revision {
            return Err(ObservationFreshnessError::Stale {
                observed: observation.revision(),
                current: current_revision,
            });
        }
        Ok(())
    }

    pub fn apply_transaction(
        &mut self,
        transaction: DocumentTransaction,
    ) -> Result<TransactionApplied, EngineError> {
        let applied = self.engine.apply_transaction(transaction)?;
        self.revision = Some(applied.new_revision);
        Ok(applied)
    }
}

#[cfg(test)]
mod tests {
    use document_protocol::{
        DocumentCapability, EngineCapabilities, ProtocolError, ProtocolVersion, TextEdit,
        TextOffset, TransactionLimits,
    };

    use super::*;

    const TEST_LIMITS: TransactionLimits = TransactionLimits::new(16, 1024, 4096);

    #[derive(Default)]
    struct TestEngine {
        text: Option<String>,
        revision: DocumentRevision,
        fail_next_open: bool,
    }

    impl DocumentEngine for TestEngine {
        fn capabilities(&self) -> EngineCapabilities {
            EngineCapabilities {
                protocol: ProtocolVersion::V0,
                capabilities: vec![
                    DocumentCapability::Read,
                    DocumentCapability::EditText,
                    DocumentCapability::SemanticSnapshot,
                ],
            }
        }

        fn revision(&self) -> Result<DocumentRevision, EngineError> {
            self.text.as_ref().ok_or(EngineError::NotOpen)?;
            Ok(self.revision)
        }

        fn open_text_fixture(&mut self, text: String) -> Result<DocumentRevision, EngineError> {
            if self.fail_next_open {
                self.fail_next_open = false;
                return Err(EngineError::Internal(String::from("injected open failure")));
            }
            self.text = Some(text);
            self.revision = DocumentRevision::INITIAL;
            Ok(self.revision)
        }

        fn semantic_text(&self) -> Result<SemanticObservation<String>, EngineError> {
            let value = self.text.clone().ok_or(EngineError::NotOpen)?;
            Ok(SemanticObservation::new(self.revision, value))
        }

        fn apply_transaction(
            &mut self,
            transaction: DocumentTransaction,
        ) -> Result<TransactionApplied, EngineError> {
            if transaction.expected_revision != self.revision {
                return Err(ProtocolError::RevisionConflict {
                    expected: transaction.expected_revision,
                    actual: self.revision,
                }
                .into());
            }

            let text = self.text.as_mut().ok_or(EngineError::NotOpen)?;
            transaction.validate_against(text, TEST_LIMITS)?;
            let mut edits = transaction.edits;
            edits.sort_by_key(|edit| edit.start_utf8);
            for edit in edits.into_iter().rev() {
                let range = edit.byte_range(text)?;
                text.replace_range(range, &edit.replacement);
            }

            let previous_revision = self.revision;
            self.revision = self.revision.next();
            Ok(TransactionApplied {
                previous_revision,
                new_revision: self.revision,
            })
        }
    }

    fn replace_first_character(expected_revision: DocumentRevision) -> DocumentTransaction {
        DocumentTransaction {
            expected_revision,
            edits: vec![TextEdit {
                start_utf8: TextOffset::new(0),
                end_utf8: TextOffset::new(1),
                replacement: String::from("A"),
            }],
        }
    }

    #[test]
    fn semantic_read_is_stamped_with_session_authority_and_revision() {
        let mut session = DocumentSession::new(TestEngine::default());
        assert_eq!(session.known_authority_generation(), None);
        session.open_text_fixture(String::from("abc")).unwrap();
        let generation = session.known_authority_generation().unwrap();

        let observation = session.semantic_text().unwrap();

        assert_eq!(observation.authority_generation(), generation);
        assert_eq!(observation.revision(), DocumentRevision::INITIAL);
        assert_eq!(observation.value().as_str(), "abc");
        assert_eq!(session.require_current(&observation), Ok(()));

        let length = observation.map(|text| text.len());
        assert_eq!(length.authority_generation(), generation);
        assert_eq!(length.revision(), DocumentRevision::INITIAL);
        assert_eq!(*length.value(), 3);
        assert_eq!(session.require_current(&length), Ok(()));
    }

    #[test]
    fn retained_observation_is_rejected_after_authoritative_mutation() {
        let mut session = DocumentSession::new(TestEngine::default());
        session.open_text_fixture(String::from("abc")).unwrap();
        let generation = session.known_authority_generation().unwrap();
        let old = session.semantic_text().unwrap();

        session
            .apply_transaction(replace_first_character(DocumentRevision::INITIAL))
            .unwrap();

        assert_eq!(session.known_authority_generation(), Some(generation));
        assert_eq!(
            session.require_current(&old),
            Err(ObservationFreshnessError::Stale {
                observed: DocumentRevision::INITIAL,
                current: DocumentRevision::new(1),
            })
        );

        let current = session.semantic_text().unwrap();
        assert_eq!(current.authority_generation(), generation);
        assert_eq!(current.revision(), DocumentRevision::new(1));
        assert_eq!(current.value().as_str(), "Abc");
        assert_eq!(session.require_current(&current), Ok(()));
    }

    #[test]
    fn old_r0_observation_is_rejected_after_successful_reopen_at_r0() {
        let mut session = DocumentSession::new(TestEngine::default());
        session.open_text_fixture(String::from("first")).unwrap();
        let first_generation = session.known_authority_generation().unwrap();
        let old = session.semantic_text().unwrap();
        assert_eq!(old.revision(), DocumentRevision::INITIAL);

        let reopened_revision = session.open_text_fixture(String::from("second")).unwrap();
        let second_generation = session.known_authority_generation().unwrap();

        assert_eq!(reopened_revision, DocumentRevision::INITIAL);
        assert_ne!(first_generation, second_generation);
        assert_eq!(
            session.require_current(&old),
            Err(ObservationFreshnessError::AuthorityChanged {
                observed: first_generation,
                current: second_generation,
            })
        );

        let current = session.semantic_text().unwrap();
        assert_eq!(current.authority_generation(), second_generation);
        assert_eq!(current.revision(), DocumentRevision::INITIAL);
        assert_eq!(current.value().as_str(), "second");
        assert_eq!(session.require_current(&current), Ok(()));
    }

    #[test]
    fn rejected_transaction_does_not_invalidate_current_observation() {
        let mut session = DocumentSession::new(TestEngine::default());
        session.open_text_fixture(String::from("abc")).unwrap();
        let generation = session.known_authority_generation().unwrap();
        let observation = session.semantic_text().unwrap();

        let error = session
            .apply_transaction(DocumentTransaction {
                expected_revision: DocumentRevision::new(99),
                edits: Vec::new(),
            })
            .unwrap_err();

        assert!(matches!(
            error,
            EngineError::Protocol(ProtocolError::RevisionConflict { .. })
        ));
        assert_eq!(session.known_revision(), Some(DocumentRevision::INITIAL));
        assert_eq!(session.known_authority_generation(), Some(generation));
        assert_eq!(session.require_current(&observation), Ok(()));
    }

    #[test]
    fn failed_open_does_not_replace_current_authority() {
        let mut session = DocumentSession::new(TestEngine::default());
        session.open_text_fixture(String::from("abc")).unwrap();
        let generation = session.known_authority_generation().unwrap();
        let observation = session.semantic_text().unwrap();
        session.engine.fail_next_open = true;

        let error = session.open_text_fixture(String::from("replacement")).unwrap_err();

        assert!(matches!(error, EngineError::Internal(message) if message.contains("injected")));
        assert_eq!(session.known_revision(), Some(DocumentRevision::INITIAL));
        assert_eq!(session.known_authority_generation(), Some(generation));
        assert_eq!(session.require_current(&observation), Ok(()));
        assert_eq!(session.semantic_text().unwrap().value().as_str(), "abc");
    }

    #[test]
    fn semantic_read_rejects_engine_session_revision_disagreement() {
        let mut session = DocumentSession::new(TestEngine::default());
        session.open_text_fixture(String::from("abc")).unwrap();
        session.engine.revision = DocumentRevision::new(5);

        let error = session.semantic_text().unwrap_err();

        assert!(matches!(error, EngineError::Internal(message) if message.contains("disagrees")));
    }
}
