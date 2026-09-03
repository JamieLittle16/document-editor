#![doc = "Deterministic in-memory engine used to prove session semantics without heavyweight dependencies."]

use document_engine_api::{DocumentEngine, EngineError};
use document_protocol::{
    DocumentCapability, DocumentRevision, DocumentTransaction, EngineCapabilities, ProtocolError,
    ProtocolVersion, TransactionApplied,
};

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

    fn semantic_text(&self) -> Result<String, EngineError> {
        self.document.clone().ok_or(EngineError::NotOpen)
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
        let mut edits = transaction.edits;
        edits.sort_by_key(|edit| edit.start_utf8);

        for pair in edits.windows(2) {
            if pair[0].end_utf8 > pair[1].start_utf8 {
                return Err(ProtocolError::InvalidRange.into());
            }
        }
        for edit in &edits {
            edit.validate(document)?;
        }
        for edit in edits.into_iter().rev() {
            document.replace_range(edit.start_utf8..edit.end_utf8, &edit.replacement);
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
    use document_protocol::{DocumentRevision, DocumentTransaction, ProtocolError, TextEdit};

    use super::*;

    #[test]
    fn transaction_advances_revision_atomically() {
        let mut engine = MockDocumentEngine::default();
        engine.open_text_fixture("hello world".into()).unwrap();

        let result = engine
            .apply_transaction(DocumentTransaction {
                expected_revision: DocumentRevision::INITIAL,
                edits: vec![TextEdit {
                    start_utf8: 6,
                    end_utf8: 11,
                    replacement: "editor".into(),
                }],
            })
            .unwrap();

        assert_eq!(result.new_revision, DocumentRevision::new(1));
        assert_eq!(engine.semantic_text().unwrap(), "hello editor");
    }

    #[test]
    fn stale_transaction_is_rejected() {
        let mut engine = MockDocumentEngine::default();
        engine.open_text_fixture("abc".into()).unwrap();
        engine
            .apply_transaction(DocumentTransaction {
                expected_revision: DocumentRevision::INITIAL,
                edits: vec![TextEdit {
                    start_utf8: 0,
                    end_utf8: 1,
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
}
