use std::collections::BTreeSet;
use std::fmt;
use std::str;

use crate::{
    DocumentLineageId, ParagraphAnchorId, ParagraphAnchorRecord, ParagraphAnchorSequence,
    ParagraphAnchorSnapshot, ParagraphAnchorTable,
};

const SNAPSHOT_MAGIC: [u8; 8] = *b"OFANCHR1";
pub const SNAPSHOT_FORMAT_VERSION: u16 = 1;

/// Explicit admission limits for decoding or encoding a durable anchor snapshot artifact.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AnchorSnapshotLimits {
    max_paragraphs: u32,
    max_paragraph_bytes: u32,
    max_total_text_bytes: u64,
}

impl AnchorSnapshotLimits {
    #[must_use]
    pub const fn new(
        max_paragraphs: u32,
        max_paragraph_bytes: u32,
        max_total_text_bytes: u64,
    ) -> Self {
        Self {
            max_paragraphs,
            max_paragraph_bytes,
            max_total_text_bytes,
        }
    }
}

/// Why a durable anchor snapshot artifact could not be decoded or emitted safely.
#[derive(Debug, Eq, PartialEq)]
pub enum AnchorSnapshotCodecError {
    InvalidMagic,
    UnsupportedVersion(u16),
    UnsupportedFlags(u16),
    Truncated,
    TrailingBytes,
    ParagraphLimitExceeded {
        actual: u32,
        limit: u32,
    },
    ParagraphBytesLimitExceeded {
        actual: u32,
        limit: u32,
    },
    TotalTextBytesLimitExceeded {
        actual: u64,
        limit: u64,
    },
    InvalidUtf8,
    ZeroSequence,
    DuplicateSequence(ParagraphAnchorSequence),
    NextSequenceNotAfterLiveAnchors {
        next: ParagraphAnchorSequence,
        greatest_live: ParagraphAnchorSequence,
    },
    LengthOverflow,
}

impl fmt::Display for AnchorSnapshotCodecError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidMagic => formatter.write_str("invalid anchor snapshot magic"),
            Self::UnsupportedVersion(version) => {
                write!(formatter, "unsupported anchor snapshot version {version}")
            }
            Self::UnsupportedFlags(flags) => {
                write!(formatter, "unsupported anchor snapshot flags {flags:#06x}")
            }
            Self::Truncated => formatter.write_str("truncated anchor snapshot"),
            Self::TrailingBytes => formatter.write_str("anchor snapshot contains trailing bytes"),
            Self::ParagraphLimitExceeded { actual, limit } => write!(
                formatter,
                "anchor snapshot paragraph count {actual} exceeds limit {limit}"
            ),
            Self::ParagraphBytesLimitExceeded { actual, limit } => write!(
                formatter,
                "anchor paragraph contains {actual} bytes, exceeding limit {limit}"
            ),
            Self::TotalTextBytesLimitExceeded { actual, limit } => write!(
                formatter,
                "anchor snapshot contains {actual} semantic text bytes, exceeding limit {limit}"
            ),
            Self::InvalidUtf8 => formatter.write_str("anchor snapshot contains invalid UTF-8"),
            Self::ZeroSequence => formatter.write_str("anchor sequence zero is reserved"),
            Self::DuplicateSequence(sequence) => write!(
                formatter,
                "duplicate live paragraph anchor sequence {sequence}"
            ),
            Self::NextSequenceNotAfterLiveAnchors {
                next,
                greatest_live,
            } => write!(
                formatter,
                "next anchor sequence {next} does not follow greatest live sequence {greatest_live}"
            ),
            Self::LengthOverflow => {
                formatter.write_str("anchor snapshot length arithmetic overflow")
            }
        }
    }
}

impl std::error::Error for AnchorSnapshotCodecError {}

/// Why a persisted anchor snapshot cannot be rebound to a current semantic projection without
/// guessing.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AnchorRebindError {
    LineageMismatch {
        expected: DocumentLineageId,
        actual: DocumentLineageId,
    },
    ParagraphCountChanged {
        expected: usize,
        actual: usize,
    },
    SemanticMismatch {
        index: usize,
    },
}

impl fmt::Display for AnchorRebindError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::LineageMismatch { expected, actual } => write!(
                formatter,
                "anchor snapshot lineage {actual} does not match expected lineage {expected}"
            ),
            Self::ParagraphCountChanged { expected, actual } => write!(
                formatter,
                "anchor snapshot expected {expected} paragraphs, current projection has {actual}"
            ),
            Self::SemanticMismatch { index } => write!(
                formatter,
                "anchor snapshot semantic evidence differs at paragraph index {index}"
            ),
        }
    }
}

impl std::error::Error for AnchorRebindError {}

impl ParagraphAnchorSnapshot {
    /// Rebind an exact semantic projection known to belong to this persisted document lineage.
    ///
    /// This method deliberately performs no candidate search or fuzzy matching. A mismatch is an
    /// unresolved reconciliation problem, not permission to guess an identity.
    pub fn rebind_exact_projection(
        &self,
        expected_lineage: DocumentLineageId,
        semantic_paragraphs: &[String],
    ) -> Result<ParagraphAnchorTable, AnchorRebindError> {
        if self.lineage != expected_lineage {
            return Err(AnchorRebindError::LineageMismatch {
                expected: expected_lineage,
                actual: self.lineage,
            });
        }
        if self.paragraphs.len() != semantic_paragraphs.len() {
            return Err(AnchorRebindError::ParagraphCountChanged {
                expected: self.paragraphs.len(),
                actual: semantic_paragraphs.len(),
            });
        }

        for (index, (record, semantic_text)) in
            self.paragraphs.iter().zip(semantic_paragraphs).enumerate()
        {
            if record.semantic_text().as_bytes() != semantic_text.as_bytes() {
                return Err(AnchorRebindError::SemanticMismatch { index });
            }
        }

        Ok(ParagraphAnchorTable {
            lineage: self.lineage,
            next_sequence: self.next_sequence,
            paragraphs: self.paragraphs.clone(),
        })
    }

    pub fn encode(
        &self,
        limits: AnchorSnapshotLimits,
    ) -> Result<Vec<u8>, AnchorSnapshotCodecError> {
        validate_snapshot(self, limits)?;
        let paragraph_count = u32::try_from(self.paragraphs.len())
            .map_err(|_| AnchorSnapshotCodecError::LengthOverflow)?;
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&SNAPSHOT_MAGIC);
        bytes.extend_from_slice(&SNAPSHOT_FORMAT_VERSION.to_le_bytes());
        bytes.extend_from_slice(&0_u16.to_le_bytes());
        bytes.extend_from_slice(self.lineage.as_bytes());
        bytes.extend_from_slice(&self.next_sequence.0.to_le_bytes());
        bytes.extend_from_slice(&paragraph_count.to_le_bytes());

        for record in &self.paragraphs {
            let semantic_bytes = record.semantic_text().as_bytes();
            let semantic_len = u32::try_from(semantic_bytes.len())
                .map_err(|_| AnchorSnapshotCodecError::LengthOverflow)?;
            bytes.extend_from_slice(&record.id().sequence().0.to_le_bytes());
            bytes.extend_from_slice(&semantic_len.to_le_bytes());
            bytes.extend_from_slice(semantic_bytes);
        }
        Ok(bytes)
    }

    pub fn decode(
        bytes: &[u8],
        limits: AnchorSnapshotLimits,
    ) -> Result<Self, AnchorSnapshotCodecError> {
        let mut reader = SnapshotReader::new(bytes);
        if reader.take_array::<8>()? != SNAPSHOT_MAGIC {
            return Err(AnchorSnapshotCodecError::InvalidMagic);
        }

        let version = u16::from_le_bytes(reader.take_array::<2>()?);
        if version != SNAPSHOT_FORMAT_VERSION {
            return Err(AnchorSnapshotCodecError::UnsupportedVersion(version));
        }
        let flags = u16::from_le_bytes(reader.take_array::<2>()?);
        if flags != 0 {
            return Err(AnchorSnapshotCodecError::UnsupportedFlags(flags));
        }

        let lineage = DocumentLineageId::from_bytes(reader.take_array::<16>()?);
        let next_sequence = read_nonzero_sequence(&mut reader)?;
        let paragraph_count = u32::from_le_bytes(reader.take_array::<4>()?);
        if paragraph_count > limits.max_paragraphs {
            return Err(AnchorSnapshotCodecError::ParagraphLimitExceeded {
                actual: paragraph_count,
                limit: limits.max_paragraphs,
            });
        }

        let capacity = usize::try_from(paragraph_count)
            .map_err(|_| AnchorSnapshotCodecError::LengthOverflow)?;
        let mut paragraphs = Vec::with_capacity(capacity);
        let mut live_sequences = BTreeSet::new();
        let mut total_text_bytes = 0_u64;
        let mut greatest_live: Option<ParagraphAnchorSequence> = None;

        for _ in 0..paragraph_count {
            let sequence = read_nonzero_sequence(&mut reader)?;
            if !live_sequences.insert(sequence) {
                return Err(AnchorSnapshotCodecError::DuplicateSequence(sequence));
            }
            greatest_live = Some(greatest_live.map_or(sequence, |current| current.max(sequence)));

            let semantic_len = u32::from_le_bytes(reader.take_array::<4>()?);
            validate_semantic_length(semantic_len, &mut total_text_bytes, limits)?;
            let semantic_len = usize::try_from(semantic_len)
                .map_err(|_| AnchorSnapshotCodecError::LengthOverflow)?;
            let semantic_bytes = reader.take_slice(semantic_len)?;
            let semantic_text = str::from_utf8(semantic_bytes)
                .map_err(|_| AnchorSnapshotCodecError::InvalidUtf8)?
                .to_owned();
            paragraphs.push(ParagraphAnchorRecord::new(
                ParagraphAnchorId::new(lineage, sequence),
                semantic_text,
            ));
        }

        if !reader.is_finished() {
            return Err(AnchorSnapshotCodecError::TrailingBytes);
        }
        validate_next_sequence(next_sequence, greatest_live)?;

        let snapshot = Self {
            lineage,
            next_sequence,
            paragraphs,
        };
        validate_snapshot(&snapshot, limits)?;
        Ok(snapshot)
    }
}

fn read_nonzero_sequence(
    reader: &mut SnapshotReader<'_>,
) -> Result<ParagraphAnchorSequence, AnchorSnapshotCodecError> {
    let raw = u64::from_le_bytes(reader.take_array::<8>()?);
    if raw == 0 {
        return Err(AnchorSnapshotCodecError::ZeroSequence);
    }
    Ok(ParagraphAnchorSequence(raw))
}

fn validate_semantic_length(
    semantic_len: u32,
    total_text_bytes: &mut u64,
    limits: AnchorSnapshotLimits,
) -> Result<(), AnchorSnapshotCodecError> {
    if semantic_len > limits.max_paragraph_bytes {
        return Err(AnchorSnapshotCodecError::ParagraphBytesLimitExceeded {
            actual: semantic_len,
            limit: limits.max_paragraph_bytes,
        });
    }
    *total_text_bytes = total_text_bytes
        .checked_add(u64::from(semantic_len))
        .ok_or(AnchorSnapshotCodecError::LengthOverflow)?;
    if *total_text_bytes > limits.max_total_text_bytes {
        return Err(AnchorSnapshotCodecError::TotalTextBytesLimitExceeded {
            actual: *total_text_bytes,
            limit: limits.max_total_text_bytes,
        });
    }
    Ok(())
}

fn validate_next_sequence(
    next_sequence: ParagraphAnchorSequence,
    greatest_live: Option<ParagraphAnchorSequence>,
) -> Result<(), AnchorSnapshotCodecError> {
    if let Some(greatest_live) = greatest_live {
        if next_sequence <= greatest_live {
            return Err(AnchorSnapshotCodecError::NextSequenceNotAfterLiveAnchors {
                next: next_sequence,
                greatest_live,
            });
        }
    }
    Ok(())
}

fn validate_snapshot(
    snapshot: &ParagraphAnchorSnapshot,
    limits: AnchorSnapshotLimits,
) -> Result<(), AnchorSnapshotCodecError> {
    let paragraph_count = u32::try_from(snapshot.paragraphs.len())
        .map_err(|_| AnchorSnapshotCodecError::LengthOverflow)?;
    if paragraph_count > limits.max_paragraphs {
        return Err(AnchorSnapshotCodecError::ParagraphLimitExceeded {
            actual: paragraph_count,
            limit: limits.max_paragraphs,
        });
    }

    let mut live_sequences = BTreeSet::new();
    let mut greatest_live: Option<ParagraphAnchorSequence> = None;
    let mut total_text_bytes = 0_u64;
    for record in &snapshot.paragraphs {
        let sequence = record.id().sequence();
        if sequence.0 == 0 || snapshot.next_sequence.0 == 0 {
            return Err(AnchorSnapshotCodecError::ZeroSequence);
        }
        if !live_sequences.insert(sequence) {
            return Err(AnchorSnapshotCodecError::DuplicateSequence(sequence));
        }
        greatest_live = Some(greatest_live.map_or(sequence, |current| current.max(sequence)));

        let semantic_len = u32::try_from(record.semantic_text().len())
            .map_err(|_| AnchorSnapshotCodecError::LengthOverflow)?;
        validate_semantic_length(semantic_len, &mut total_text_bytes, limits)?;
    }
    validate_next_sequence(snapshot.next_sequence, greatest_live)
}

struct SnapshotReader<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> SnapshotReader<'a> {
    const fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, offset: 0 }
    }

    fn take_array<const N: usize>(&mut self) -> Result<[u8; N], AnchorSnapshotCodecError> {
        self.take_slice(N)?
            .try_into()
            .map_err(|_| AnchorSnapshotCodecError::Truncated)
    }

    fn take_slice(&mut self, length: usize) -> Result<&'a [u8], AnchorSnapshotCodecError> {
        let end = self
            .offset
            .checked_add(length)
            .ok_or(AnchorSnapshotCodecError::LengthOverflow)?;
        let Some(slice) = self.bytes.get(self.offset..end) else {
            return Err(AnchorSnapshotCodecError::Truncated);
        };
        self.offset = end;
        Ok(slice)
    }

    const fn is_finished(&self) -> bool {
        self.offset == self.bytes.len()
    }
}
