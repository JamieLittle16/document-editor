#![doc = "Versioned, wire-safe value types shared across the document-engine boundary."]

use std::fmt;
use std::ops::Range;

/// Correlates one request with its response across an engine transport.
///
/// Fixed-width by design: protocol values must not depend on host pointer width.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct RequestId(u64);

impl RequestId {
    #[must_use]
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

impl fmt::Display for RequestId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

/// Monotonic engine revision for one authoritative document session.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
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
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

/// UTF-8 byte offset used by protocol text operations.
///
/// This is deliberately `u64` rather than `usize`: serialized protocol values must have
/// identical width on 32-bit and 64-bit hosts. Engines convert to local indices only after
/// validating the offset against the current document text.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct TextOffset(u64);

impl TextOffset {
    #[must_use]
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

impl fmt::Display for TextOffset {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
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

/// One replacement over a UTF-8 byte range in a known document revision.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TextEdit {
    pub start_utf8: TextOffset,
    pub end_utf8: TextOffset,
    pub replacement: String,
}

impl TextEdit {
    /// Validates this edit against concrete UTF-8 text and returns native byte indices.
    pub fn byte_range(&self, text: &str) -> Result<Range<usize>, ProtocolError> {
        let text_len = u64::try_from(text.len()).map_err(|_| ProtocolError::DocumentTooLarge)?;
        if self.start_utf8 > self.end_utf8 || self.end_utf8.get() > text_len {
            return Err(ProtocolError::InvalidRange);
        }

        let start = usize::try_from(self.start_utf8.get()).map_err(|_| ProtocolError::InvalidRange)?;
        let end = usize::try_from(self.end_utf8.get()).map_err(|_| ProtocolError::InvalidRange)?;
        if !text.is_char_boundary(start) || !text.is_char_boundary(end) {
            return Err(ProtocolError::NotACharacterBoundary);
        }
        Ok(start..end)
    }
}

/// Explicit admission limits for one document transaction.
///
/// Limits are chosen by the engine/session policy rather than hidden inside protocol parsing.
/// This keeps the wire contract bounded while allowing later product profiles to qualify
/// different limits without changing the transaction algebra.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TransactionLimits {
    max_edits: u32,
    max_replacement_bytes: u64,
    max_total_replacement_bytes: u64,
}

impl TransactionLimits {
    #[must_use]
    pub const fn new(
        max_edits: u32,
        max_replacement_bytes: u64,
        max_total_replacement_bytes: u64,
    ) -> Self {
        Self {
            max_edits,
            max_replacement_bytes,
            max_total_replacement_bytes,
        }
    }

    #[must_use]
    pub const fn max_edits(self) -> u32 {
        self.max_edits
    }

    #[must_use]
    pub const fn max_replacement_bytes(self) -> u64 {
        self.max_replacement_bytes
    }

    #[must_use]
    pub const fn max_total_replacement_bytes(self) -> u64 {
        self.max_total_replacement_bytes
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DocumentTransaction {
    pub expected_revision: DocumentRevision,
    pub edits: Vec<TextEdit>,
}

impl DocumentTransaction {
    /// Validates resource bounds, UTF-8 ranges and non-overlap against a source snapshot.
    ///
    /// No mutation may begin until this succeeds, preserving all-or-nothing transaction
    /// admission for engines that implement the protocol.
    pub fn validate_against(
        &self,
        text: &str,
        limits: TransactionLimits,
    ) -> Result<(), ProtocolError> {
        let edit_count = u64::try_from(self.edits.len()).unwrap_or(u64::MAX);
        if edit_count > u64::from(limits.max_edits) {
            return Err(ProtocolError::TooManyEdits {
                actual: edit_count,
                max: limits.max_edits,
            });
        }

        let mut total_replacement_bytes = 0_u64;
        let mut ordered = Vec::with_capacity(self.edits.len());

        for edit in &self.edits {
            let replacement_bytes = u64::try_from(edit.replacement.len())
                .map_err(|_| ProtocolError::TransactionPayloadTooLarge {
                    actual: u64::MAX,
                    max: limits.max_total_replacement_bytes,
                })?;
            if replacement_bytes > limits.max_replacement_bytes {
                return Err(ProtocolError::ReplacementTooLarge {
                    actual: replacement_bytes,
                    max: limits.max_replacement_bytes,
                });
            }
            total_replacement_bytes = total_replacement_bytes
                .checked_add(replacement_bytes)
                .ok_or(ProtocolError::TransactionPayloadTooLarge {
                    actual: u64::MAX,
                    max: limits.max_total_replacement_bytes,
                })?;
            if total_replacement_bytes > limits.max_total_replacement_bytes {
                return Err(ProtocolError::TransactionPayloadTooLarge {
                    actual: total_replacement_bytes,
                    max: limits.max_total_replacement_bytes,
                });
            }

            edit.byte_range(text)?;
            ordered.push(edit);
        }

        ordered.sort_by_key(|edit| edit.start_utf8);
        for pair in ordered.windows(2) {
            if pair[0].end_utf8 > pair[1].start_utf8 {
                return Err(ProtocolError::InvalidRange);
            }
        }

        Ok(())
    }
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
    DocumentTooLarge,
    TooManyEdits {
        actual: u64,
        max: u32,
    },
    ReplacementTooLarge {
        actual: u64,
        max: u64,
    },
    TransactionPayloadTooLarge {
        actual: u64,
        max: u64,
    },
    RevisionConflict {
        expected: DocumentRevision,
        actual: DocumentRevision,
    },
}

impl fmt::Display for ProtocolError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidRange => formatter.write_str("invalid text range"),
            Self::NotACharacterBoundary => {
                formatter.write_str("range is not on a UTF-8 character boundary")
            }
            Self::DocumentTooLarge => {
                formatter.write_str("document text is too large for protocol UTF-8 offsets")
            }
            Self::TooManyEdits { actual, max } => {
                write!(formatter, "transaction contains {actual} edits; limit is {max}")
            }
            Self::ReplacementTooLarge { actual, max } => write!(
                formatter,
                "one text replacement contains {actual} bytes; limit is {max}"
            ),
            Self::TransactionPayloadTooLarge { actual, max } => write!(
                formatter,
                "transaction replacement payload contains {actual} bytes; limit is {max}"
            ),
            Self::RevisionConflict { expected, actual } => {
                write!(formatter, "revision conflict: expected {expected}, actual {actual}")
            }
        }
    }
}

impl std::error::Error for ProtocolError {}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_LIMITS: TransactionLimits = TransactionLimits::new(4, 8, 12);

    fn offset(value: u64) -> TextOffset {
        TextOffset::new(value)
    }

    #[test]
    fn text_edit_rejects_non_character_boundary() {
        let text = "aéz";
        let edit = TextEdit {
            start_utf8: offset(2),
            end_utf8: offset(3),
            replacement: String::new(),
        };
        assert_eq!(
            edit.byte_range(text),
            Err(ProtocolError::NotACharacterBoundary)
        );
    }

    #[test]
    fn text_edit_rejects_fixed_width_offset_beyond_document() {
        let edit = TextEdit {
            start_utf8: offset(u64::MAX),
            end_utf8: offset(u64::MAX),
            replacement: String::new(),
        };
        assert_eq!(edit.byte_range("abc"), Err(ProtocolError::InvalidRange));
    }

    #[test]
    fn transaction_rejects_overlapping_ranges_before_mutation() {
        let transaction = DocumentTransaction {
            expected_revision: DocumentRevision::INITIAL,
            edits: vec![
                TextEdit {
                    start_utf8: offset(0),
                    end_utf8: offset(2),
                    replacement: "x".into(),
                },
                TextEdit {
                    start_utf8: offset(1),
                    end_utf8: offset(3),
                    replacement: "y".into(),
                },
            ],
        };

        assert_eq!(
            transaction.validate_against("abcd", TEST_LIMITS),
            Err(ProtocolError::InvalidRange)
        );
    }

    #[test]
    fn transaction_rejects_excess_edit_count() {
        let edits = (0..5)
            .map(|_| TextEdit {
                start_utf8: offset(0),
                end_utf8: offset(0),
                replacement: String::new(),
            })
            .collect();
        let transaction = DocumentTransaction {
            expected_revision: DocumentRevision::INITIAL,
            edits,
        };

        assert_eq!(
            transaction.validate_against("", TEST_LIMITS),
            Err(ProtocolError::TooManyEdits { actual: 5, max: 4 })
        );
    }

    #[test]
    fn transaction_rejects_single_large_replacement() {
        let transaction = DocumentTransaction {
            expected_revision: DocumentRevision::INITIAL,
            edits: vec![TextEdit {
                start_utf8: offset(0),
                end_utf8: offset(0),
                replacement: "123456789".into(),
            }],
        };

        assert_eq!(
            transaction.validate_against("", TEST_LIMITS),
            Err(ProtocolError::ReplacementTooLarge { actual: 9, max: 8 })
        );
    }

    #[test]
    fn transaction_rejects_large_aggregate_replacement_payload() {
        let transaction = DocumentTransaction {
            expected_revision: DocumentRevision::INITIAL,
            edits: vec![
                TextEdit {
                    start_utf8: offset(0),
                    end_utf8: offset(0),
                    replacement: "1234567".into(),
                },
                TextEdit {
                    start_utf8: offset(1),
                    end_utf8: offset(1),
                    replacement: "7654321".into(),
                },
            ],
        };

        assert_eq!(
            transaction.validate_against("ab", TEST_LIMITS),
            Err(ProtocolError::TransactionPayloadTooLarge { actual: 14, max: 12 })
        );
    }

    #[test]
    fn valid_transaction_passes_structural_and_text_validation() {
        let transaction = DocumentTransaction {
            expected_revision: DocumentRevision::INITIAL,
            edits: vec![TextEdit {
                start_utf8: offset(1),
                end_utf8: offset(3),
                replacement: "xy".into(),
            }],
        };

        assert_eq!(transaction.validate_against("abcd", TEST_LIMITS), Ok(()));
    }
}
