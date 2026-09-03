#![doc = "Versioned value types shared across the document-engine boundary."]

use std::fmt;

#[derive(Clone, Copy, Debug, Default, Eq, Ord, PartialEq, PartialOrd)]
pub struct DocumentRevision(u64);

impl DocumentRevision {
    pub const INITIAL: Self = Self(0);

    #[must_use]
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }

    #[must_use]
    pub const fn next(self) -> Self {
        Self(self.0.saturating_add(1))
    }
}

impl fmt::Display for DocumentRevision {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProtocolVersion {
    pub major: u16,
    pub minor: u16,
}

impl ProtocolVersion {
    pub const V0: Self = Self { major: 0, minor: 1 };
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DocumentCapability {
    Read,
    EditText,
    Render,
    Save,
    SemanticSnapshot,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EngineCapabilities {
    pub protocol: ProtocolVersion,
    pub capabilities: Vec<DocumentCapability>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TextEdit {
    pub start_utf8: usize,
    pub end_utf8: usize,
    pub replacement: String,
}

impl TextEdit {
    pub fn validate(&self, text: &str) -> Result<(), ProtocolError> {
        if self.start_utf8 > self.end_utf8 || self.end_utf8 > text.len() {
            return Err(ProtocolError::InvalidRange);
        }
        if !text.is_char_boundary(self.start_utf8) || !text.is_char_boundary(self.end_utf8) {
            return Err(ProtocolError::NotACharacterBoundary);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DocumentTransaction {
    pub expected_revision: DocumentRevision,
    pub edits: Vec<TextEdit>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TransactionApplied {
    pub previous_revision: DocumentRevision,
    pub new_revision: DocumentRevision,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProtocolError {
    InvalidRange,
    NotACharacterBoundary,
    RevisionConflict {
        expected: DocumentRevision,
        actual: DocumentRevision,
    },
}

impl fmt::Display for ProtocolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidRange => f.write_str("invalid text range"),
            Self::NotACharacterBoundary => {
                f.write_str("range is not on a UTF-8 character boundary")
            }
            Self::RevisionConflict { expected, actual } => {
                write!(f, "revision conflict: expected {expected}, actual {actual}")
            }
        }
    }
}

impl std::error::Error for ProtocolError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_edit_rejects_non_character_boundary() {
        let text = "aéz";
        let edit = TextEdit {
            start_utf8: 2,
            end_utf8: 3,
            replacement: String::new(),
        };
        assert_eq!(
            edit.validate(text),
            Err(ProtocolError::NotACharacterBoundary)
        );
    }
}
