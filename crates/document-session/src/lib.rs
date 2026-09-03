#![doc = "Revision-aware orchestration around a replaceable document engine."]

use document_engine_api::{DocumentEngine, EngineError};
use document_protocol::{DocumentRevision, DocumentTransaction, TransactionApplied};

pub struct DocumentSession<E> {
    engine: E,
    revision: Option<DocumentRevision>,
}

impl<E: DocumentEngine> DocumentSession<E> {
    #[must_use]
    pub const fn new(engine: E) -> Self {
        Self { engine, revision: None }
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

    pub fn semantic_text(&self) -> Result<String, EngineError> {
        self.engine.semantic_text()
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
