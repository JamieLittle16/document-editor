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

/// Immutable semantic data observed from one exact authoritative document revision.
///
/// Semantic results must never be treated as timeless state. Engines stamp the value while
/// reading it so callers can deterministically reject results after the document advances.
/// The owning document/session supplies document identity; this type deliberately does not
/// invent a product-level `DocumentId` before multi-document authority is specified.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SemanticObservation<T> {
    revision: DocumentRevision,
    value: T,
}

impl<T> SemanticObservation<T> {
    #[must_use]
    pub fn new(revision: DocumentRevision, value: T) -> Self {
        Self { revision, value }
    }

    #[must_use]
    pub const fn revision(&self) -> DocumentRevision {
        self.revision
    }

    #[must_use]
    pub const fn value(&self) -> &T {
        &self.value
    }

    #[must_use]
    pub fn into_value(self) -> T {
        self.value
    }

    #[must_use]
    pub fn map<U>(self, map: impl FnOnce(T) -> U) -> SemanticObservation<U> {
        SemanticObservation::new(self.revision, map(self.value))
    }
}

pub trait DocumentEngine {
    fn capabilities(&self) -> EngineCapabilities;
    fn revision(&self) -> Result<DocumentRevision, EngineError>;
    fn open_text_fixture(&mut self, text: String) -> Result<DocumentRevision, EngineError>;
    fn semantic_text(&self) -> Result<SemanticObservation<String>, EngineError>;
    fn apply_transaction(
        &mut self,
        transaction: DocumentTransaction,
    ) -> Result<TransactionApplied, EngineError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn semantic_observation_map_preserves_revision() {
        let observed = SemanticObservation::new(DocumentRevision::new(7), String::from("abc"));
        let length = observed.map(|text| text.len());

        assert_eq!(length.revision(), DocumentRevision::new(7));
        assert_eq!(*length.value(), 3);
    }
}
