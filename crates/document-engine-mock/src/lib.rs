#![doc = "Deterministic in-memory engine used to prove session semantics without heavyweight dependencies."]

use document_engine_api::{DocumentEngine, EngineError, SemanticObservation};
use document_protocol::{
    DocumentCapability, DocumentRevision, DocumentTransaction, EngineCapabilities, ProtocolError,
    ProtocolVersion, TransactionApplied, TransactionLimits,
};

/// R0A mock admission policy. Production engines will qualify their own explicit limits.
const MOCK_TRANSACTION_LIMITS: TransactionLimits =
    TransactionLimits::new(4096, 16 * 1024 * 1024, 64 * 1024 * 1024);

#[derive(Default)]
pub struct MockDocumentEngine {
    document: Option<String>,
    revision: DocumentRevision,
}

impl DocumentEngine for MockDocumentEngine {
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
        self.document.as_ref().ok_or(EngineError::NotOpen)?;
        Ok(self.revision)
    }

    fn open_text_fixture(&mut self, text: String) -> Result<DocumentRevision, EngineError> {
        self.document = Some(text);
        self.revision = DocumentRevision::INITIAL;
        Ok(self.revision)
    }

    fn semantic_text(&self) -> Result<SemanticObservation<String>, EngineError> {
        let value = self.document.clone().ok_or(EngineError::NotOpen)?;
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

        let document = self.document.as_mut().ok_or(EngineError::NotOpen)?;
        transaction.validate_against(document, MOCK_TRANSACTION_LIMITS)?;

        let mut edits = transaction.edits;
        edits.sort_by_key(|edit| edit.start_utf8);
        for edit in edits.into_iter().rev() {
            let range = edit.byte_range(document)?;
            document.replace_range(range, &edit.replacement);
        }

        let previous_revision = self.revision;
        self.revision = self.revision.next();
        Ok(TransactionApplied {
            previous_revision,
            new_revision: self.revision,
        })
    }
}

#[cfg(test)]
mod tests {
    use document_engine_api::DocumentEngine;
    use document_protocol::{
        DocumentRevision, DocumentTransaction, ProtocolError, TextEdit, TextOffset,
    };

    use super::*;

    fn offset(value: u64) -> TextOffset {
        TextOffset::new(value)
    }

    #[test]
    fn transaction_advances_revision_atomically() {
        let mut engine = MockDocumentEngine::default();
        engine.open_text_fixture("hello world".into()).unwrap();

        let result = engine
            .apply_transaction(DocumentTransaction {
                expected_revision: DocumentRevision::INITIAL,
                edits: vec![TextEdit {
                    start_utf8: offset(6),
                    end_utf8: offset(11),
                    replacement: "editor".into(),
                }],
            })
            .unwrap();

        assert_eq!(result.new_revision, DocumentRevision::new(1));
        let observation = engine.semantic_text().unwrap();
        assert_eq!(observation.revision(), DocumentRevision::new(1));
        assert_eq!(observation.value(), "hello editor");
    }

    #[test]
    fn semantic_observation_is_stamped_with_exact_read_revision() {
        let mut engine = MockDocumentEngine::default();
        engine.open_text_fixture("abc".into()).unwrap();

        let initial = engine.semantic_text().unwrap();
        assert_eq!(initial.revision(), DocumentRevision::INITIAL);
        assert_eq!(initial.value(), "abc");

        engine
            .apply_transaction(DocumentTransaction {
                expected_revision: DocumentRevision::INITIAL,
                edits: vec![TextEdit {
                    start_utf8: offset(0),
                    end_utf8: offset(1),
                    replacement: "A".into(),
                }],
            })
            .unwrap();

        let current = engine.semantic_text().unwrap();
        assert_eq!(current.revision(), DocumentRevision::new(1));
        assert_eq!(current.value(), "Abc");
        assert_eq!(initial.revision(), DocumentRevision::INITIAL);
        assert_eq!(initial.value(), "abc");
    }

    #[test]
    fn stale_transaction_is_rejected() {
        let mut engine = MockDocumentEngine::default();
        engine.open_text_fixture("abc".into()).unwrap();
        engine
            .apply_transaction(DocumentTransaction {
                expected_revision: DocumentRevision::INITIAL,
                edits: vec![TextEdit {
                    start_utf8: offset(0),
                    end_utf8: offset(1),
                    replacement: "A".into(),
                }],
            })
            .unwrap();

        let error = engine
            .apply_transaction(DocumentTransaction {
                expected_revision: DocumentRevision::INITIAL,
                edits: Vec::new(),
            })
            .unwrap_err();

        assert!(matches!(
            error,
            EngineError::Protocol(ProtocolError::RevisionConflict { .. })
        ));
    }

    #[test]
    fn invalid_multi_edit_transaction_changes_nothing() {
        let mut engine = MockDocumentEngine::default();
        engine.open_text_fixture("abcdef".into()).unwrap();

        let error = engine
            .apply_transaction(DocumentTransaction {
                expected_revision: DocumentRevision::INITIAL,
                edits: vec![
                    TextEdit {
                        start_utf8: offset(0),
                        end_utf8: offset(2),
                        replacement: "X".into(),
                    },
                    TextEdit {
                        start_utf8: offset(1),
                        end_utf8: offset(3),
                        replacement: "Y".into(),
                    },
                ],
            })
            .unwrap_err();

        assert!(matches!(
            error,
            EngineError::Protocol(ProtocolError::InvalidRange)
        ));
        let observation = engine.semantic_text().unwrap();
        assert_eq!(observation.value(), "abcdef");
        assert_eq!(observation.revision(), DocumentRevision::INITIAL);
        assert_eq!(engine.revision().unwrap(), DocumentRevision::INITIAL);
    }
}
