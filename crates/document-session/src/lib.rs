#![doc = "Revision-aware orchestration around a replaceable document engine."]

use std::fmt;

use document_engine_api::{DocumentEngine, EngineError, SemanticObservation};
use document_protocol::{DocumentRevision, DocumentTransaction, TransactionApplied};

/// Why a previously produced semantic observation cannot be consumed as current state.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ObservationFreshnessError {
    NoOpenDocument,
    Stale {
        observed: DocumentRevision,
        current: DocumentRevision,
    },
}

impl fmt::Display for ObservationFreshnessError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoOpenDocument => formatter.write_str("no authoritative document revision is open"),
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
    revision: Option<DocumentRevision>,
}

impl<E: DocumentEngine> DocumentSession<E> {
    #[must_use]
    pub const fn new(engine: E) -> Self {
        Self {
            engine,
            revision: None,
        }
    }

    pub fn open_text_fixture(&mut self, text: String) -> Result<DocumentRevision, EngineError> {
        let revision = self.engine.open_text_fixture(text)?;
        self.revision = Some(revision);
        Ok(revision)
    }

    #[must_use]
    pub const fn known_revision(&self) -> Option<DocumentRevision> {
        self.revision
    }

    /// Reads semantic text and verifies that the engine stamped it with this session's authority.
    ///
    /// A successful read is coherent at the instant it is returned. Callers that retain the
    /// observation across work or await points must call [`Self::require_current`] before using
    /// it to affect current UI/application state.
    pub fn semantic_text(&self) -> Result<SemanticObservation<String>, EngineError> {
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
        Ok(observation)
    }

    /// Rejects an observation that was produced before the current authoritative revision.
    ///
    /// This check is intentionally local and allocation-free. It is the primitive that future
    /// asynchronous search, diagnostics, comments and other feature modules use before applying
    /// derived semantic results to current state.
    pub fn require_current<T>(
        &self,
        observation: &SemanticObservation<T>,
    ) -> Result<(), ObservationFreshnessError> {
        let current = self
            .revision
            .ok_or(ObservationFreshnessError::NoOpenDocument)?;
        if observation.revision() == current {
            Ok(())
        } else {
            Err(ObservationFreshnessError::Stale {
                observed: observation.revision(),
                current,
            })
        }
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
    fn semantic_read_is_stamped_with_session_revision() {
        let mut session = DocumentSession::new(TestEngine::default());
        session.open_text_fixture(String::from("abc")).unwrap();

        let observation = session.semantic_text().unwrap();

        assert_eq!(observation.revision(), DocumentRevision::INITIAL);
        assert_eq!(observation.value().as_str(), "abc");
        assert_eq!(session.require_current(&observation), Ok(()));
    }

    #[test]
    fn retained_observation_is_rejected_after_authoritative_mutation() {
        let mut session = DocumentSession::new(TestEngine::default());
        session.open_text_fixture(String::from("abc")).unwrap();
        let old = session.semantic_text().unwrap();

        session
            .apply_transaction(replace_first_character(DocumentRevision::INITIAL))
            .unwrap();

        assert_eq!(
            session.require_current(&old),
            Err(ObservationFreshnessError::Stale {
                observed: DocumentRevision::INITIAL,
                current: DocumentRevision::new(1),
            })
        );

        let current = session.semantic_text().unwrap();
        assert_eq!(current.revision(), DocumentRevision::new(1));
        assert_eq!(current.value().as_str(), "Abc");
        assert_eq!(session.require_current(&current), Ok(()));
    }

    #[test]
    fn rejected_transaction_does_not_invalidate_current_observation() {
        let mut session = DocumentSession::new(TestEngine::default());
        session.open_text_fixture(String::from("abc")).unwrap();
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
        assert_eq!(session.require_current(&observation), Ok(()));
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
