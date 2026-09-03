#![doc = "Engine abstraction independent of LibreOffice or any future native engine."]

use document_protocol::{
    DocumentRevision, DocumentTransaction, EngineCapabilities, ProtocolError, TransactionApplied,
};

#[derive(Debug)]
pub enum EngineError {
    Protocol(ProtocolError),
    NotOpen,
    Unsupported(&'static str),
    Internal(String),
}

impl From<ProtocolError> for EngineError {
    fn from(value: ProtocolError) -> Self {
        Self::Protocol(value)
    }
}

pub trait DocumentEngine {
    fn capabilities(&self) -> EngineCapabilities;
    fn revision(&self) -> Result<DocumentRevision, EngineError>;
    fn open_text_fixture(&mut self, text: String) -> Result<DocumentRevision, EngineError>;
    fn semantic_text(&self) -> Result<String, EngineError>;
    fn apply_transaction(
        &mut self,
        transaction: DocumentTransaction,
    ) -> Result<TransactionApplied, EngineError>;
}
