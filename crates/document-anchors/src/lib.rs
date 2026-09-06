#![doc = "Product-owned durable paragraph anchors and conservative reconciliation artifacts."]

use std::collections::BTreeSet;
use std::fmt;
use std::str;

const SNAPSHOT_MAGIC: [u8; 8] = *b"OFANCHR1";
pub const SNAPSHOT_FORMAT_VERSION: u16 = 1;
const SNAPSHOT_HEADER_BYTES: usize = 40;
const RECORD_HEADER_BYTES: usize = 12;

/// Durable product-owned identity for one logical document lineage.
///
/// The persistence/application layer is responsible for minting and storing this value. The
/// anchor crate deliberately does not derive it from file paths, package bytes, Writer objects,
/// semantic text or file-format identifiers.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct DocumentLineageId([u8; 16]);

impl DocumentLineageId {
    #[must_use]
    pub const fn from_bytes(bytes: [u8; 16]) -> Self {
        Self(bytes)
    }

    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 16] {
        &self.0
    }
}

impl fmt::Display for DocumentLineageId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        for byte in self.0 {
            write!(formatter, "{byte:02x}")?;
        }
        Ok(())
    }
}

/// Monotonic paragraph-anchor sequence inside one durable document lineage.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ParagraphAnchorSequence(u64);

impl ParagraphAnchorSequence {
    const FIRST: Self = Self(1);

    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

impl fmt::Display for ParagraphAnchorSequence {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

/// Product-owned durable identity for one logical paragraph in a document lineage.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ParagraphAnchorId {
    lineage: DocumentLineageId,
    sequence: ParagraphAnchorSequence,
}

impl ParagraphAnchorId {
    const fn new(lineage: DocumentLineageId, sequence: ParagraphAnchorSequence) -> Self {
        Self { lineage, sequence }
    }

    #[must_use]
    pub const fn lineage(self) -> DocumentLineageId {
        self.lineage
    }

    #[must_use]
    pub const fn sequence(self) -> ParagraphAnchorSequence {
        self.sequence
    }
}

impl fmt::Display for ParagraphAnchorId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}:{}", self.lineage, self.sequence)
    }
}

/// One anchored paragraph plus semantic text retained only as reconciliation evidence.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParagraphAnchorRecord {
    id: ParagraphAnchorId,
    semantic_text: String,
}

impl ParagraphAnchorRecord {
    fn new(id: ParagraphAnchorId, semantic_text: String) -> Self {
        Self { id, semantic_text }
    }

    #[must_use]
    pub const fn id(&self) -> ParagraphAnchorId {
        self.id
    }

    #[must_use]
    pub fn semantic_text(&self) -> &str {
        &self.semantic_text
    }
}

/// Why a product-owned structural anchor mutation was rejected before changing the table.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AnchorMutationError {
    ParagraphOutOfBounds { index: usize, paragraph_count: usize },
    NoFollowingParagraph { index: usize, paragraph_count: usize },
    SequenceExhausted,
}

impl fmt::Display for AnchorMutationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ParagraphOutOfBounds {
                index,
                paragraph_count,
            } => write!(
                formatter,
                "paragraph index {index} is outside paragraph count {paragraph_count}"
            ),
            Self::NoFollowingParagraph {
                index,
                paragraph_count,
            } => write!(
                formatter,
                "paragraph index {index} has no following paragraph in count {paragraph_count}"
            ),
            Self::SequenceExhausted => formatter.write_str("paragraph anchor sequence exhausted"),
        }
    }
}

impl std::error::Error for AnchorMutationError {}

/// Identity result of a product-owned merge operation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ParagraphMergeApplied {
    surviving: ParagraphAnchorId,
    retired: ParagraphAnchorId,
}

impl ParagraphMergeApplied {
    #[must_use]
    pub const fn surviving(self) -> ParagraphAnchorId {
        self.surviving
    }

    #[must_use]
    pub const fn retired(self) -> ParagraphAnchorId {
        self.retired
    }
}

/// Live product-owned paragraph-anchor projection for one logical document lineage.
///
/// This table is intentionally independent of engine objects and native paragraph identities. It
/// is updated only by product semantic operations whose structural meaning is known. Generic byte
/// edits are not interpreted as paragraph operations here.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParagraphAnchorTable {
    lineage: DocumentLineageId,
    next_sequence: ParagraphAnchorSequence,
    paragraphs: Vec<ParagraphAnchorRecord>,
}

impl ParagraphAnchorTable {
    pub fn from_projection(
        lineage: DocumentLineageId,
        semantic_paragraphs: Vec<String>,
    ) -> Result<Self, AnchorMutationError> {
        let mut table = Self {
            lineage,
            next_sequence: ParagraphAnchorSequence::FIRST,
            paragraphs: Vec::with_capacity(semantic_paragraphs.len()),
        };

        for semantic_text in semantic_paragraphs {
            let id = table.mint_anchor()?;
            table
                .paragraphs
                .push(ParagraphAnchorRecord::new(id, semantic_text));
        }

        Ok(table)
    }

    #[must_use]
    pub const fn lineage(&self) -> DocumentLineageId {
        self.lineage
    }

    #[must_use]
    pub fn paragraphs(&self) -> &[ParagraphAnchorRecord] {
        &self.paragraphs
    }

    /// Preserve paragraph identity while replacing its semantic text.
    pub fn replace_paragraph_text(
        &mut self,
        index: usize,
        semantic_text: String,
    ) -> Result<ParagraphAnchorId, AnchorMutationError> {
        let paragraph_count = self.paragraphs.len();
        let Some(record) = self.paragraphs.get_mut(index) else {
            return Err(AnchorMutationError::ParagraphOutOfBounds {
                index,
                paragraph_count,
            });
        };
        record.semantic_text = semantic_text;
        Ok(record.id)
    }

    /// Insert a new logical paragraph and mint a never-reused product anchor for it.
    pub fn insert_paragraph(
        &mut self,
        index: usize,
        semantic_text: String,
    ) -> Result<ParagraphAnchorId, AnchorMutationError> {
        let paragraph_count = self.paragraphs.len();
        if index > paragraph_count {
            return Err(AnchorMutationError::ParagraphOutOfBounds {
                index,
                paragraph_count,
            });
        }
        let id = self.mint_anchor()?;
        self.paragraphs
            .insert(index, ParagraphAnchorRecord::new(id, semantic_text));
        Ok(id)
    }

    /// Split one logical paragraph, preserving the original product anchor on the left fragment
    /// and minting a fresh anchor for the right fragment.
    pub fn split_paragraph(
        &mut self,
        index: usize,
        left_text: String,
        right_text: String,
    ) -> Result<(ParagraphAnchorId, ParagraphAnchorId), AnchorMutationError> {
        let paragraph_count = self.paragraphs.len();
        let Some(original_id) = self.paragraphs.get(index).map(ParagraphAnchorRecord::id) else {
            return Err(AnchorMutationError::ParagraphOutOfBounds {
                index,
                paragraph_count,
            });
        };
        let right_id = self.mint_anchor()?;
        self.paragraphs[index].semantic_text = left_text;
        self.paragraphs.insert(
            index + 1,
            ParagraphAnchorRecord::new(right_id, right_text),
        );
        Ok((original_id, right_id))
    }

    /// Merge one paragraph with its following paragraph. The left product anchor survives and the
    /// right product anchor is retired permanently.
    pub fn merge_with_next(
        &mut self,
        index: usize,
        merged_text: String,
    ) -> Result<ParagraphMergeApplied, AnchorMutationError> {
        let paragraph_count = self.paragraphs.len();
        if index >= paragraph_count {
            return Err(AnchorMutationError::ParagraphOutOfBounds {
                index,
                paragraph_count,
            });
        }
        if index + 1 >= paragraph_count {
            return Err(AnchorMutationError::NoFollowingParagraph {
                index,
                paragraph_count,
            });
        }

        let surviving = self.paragraphs[index].id;
        self.paragraphs[index].semantic_text = merged_text;
        let retired = self.paragraphs.remove(index + 1).id;
        Ok(ParagraphMergeApplied {
            surviving,
            retired,
        })
    }

    /// Delete one logical paragraph and permanently retire its anchor.
    pub fn delete_paragraph(
        &mut self,
        index: usize,
    ) -> Result<ParagraphAnchorId, AnchorMutationError> {
        let paragraph_count = self.paragraphs.len();
        if index >= paragraph_count {
            return Err(AnchorMutationError::ParagraphOutOfBounds {
                index,
                paragraph_count,
            });
        }
        Ok(self.paragraphs.remove(index).id)
    }

    #[must_use]
    pub fn snapshot(&self) -> ParagraphAnchorSnapshot {
        ParagraphAnchorSnapshot {
            lineage: self.lineage,
            next_sequence: self.next_sequence,
            paragraphs: self.paragraphs.clone(),
        }
    }

    fn mint_anchor(&mut self) -> Result<ParagraphAnchorId, AnchorMutationError> {
        let sequence = self.next_sequence;
        let next = sequence
            .0
            .checked_add(1)
            .map(ParagraphAnchorSequence)
            .ok_or(AnchorMutationError::SequenceExhausted)?;
        self.next_sequence = next;
        Ok(ParagraphAnchorId::new(self.lineage, sequence))
    }
}

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
    ParagraphLimitExceeded { actual: u32, limit: u32 },
    ParagraphBytesLimitExceeded { actual: u32, limit: u32 },
    TotalTextBytesLimitExceeded { actual: u64, limit: u64 },
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
            Self::DuplicateSequence(sequence) => {
                write!(formatter, "duplicate live paragraph anchor sequence {sequence}")
            }
            Self::NextSequenceNotAfterLiveAnchors {
                next,
                greatest_live,
            } => write!(
                formatter,
                "next anchor sequence {next} does not follow greatest live sequence {greatest_live}"
            ),
            Self::LengthOverflow => formatter.write_str("anchor snapshot length arithmetic overflow"),
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

/// Bounded, versioned durable artifact for product-owned paragraph anchors.
///
/// Semantic text is stored only to verify an exact known-lineage rebind. It is never used as an
/// identity key or searched for candidate matches. If the projection differs, the operation is
/// unresolved and a richer structural/history reconciliation path must decide what happened.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParagraphAnchorSnapshot {
    lineage: DocumentLineageId,
    next_sequence: ParagraphAnchorSequence,
    paragraphs: Vec<ParagraphAnchorRecord>,
}

impl ParagraphAnchorSnapshot {
    #[must_use]
    pub const fn lineage(&self) -> DocumentLineageId {
        self.lineage
    }

    #[must_use]
    pub fn paragraphs(&self) -> &[ParagraphAnchorRecord] {
        &self.paragraphs
    }

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
        for (index, (record, semantic_text)) in self
            .paragraphs
            .iter()
            .zip(semantic_paragraphs)
            .enumerate()
        {
            if record.semantic_text != *semantic_text {
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
            let semantic_bytes = record.semantic_text.as_bytes();
            let semantic_len = u32::try_from(semantic_bytes.len())
                .map_err(|_| AnchorSnapshotCodecError::LengthOverflow)?;
            bytes.extend_from_slice(&record.id.sequence.0.to_le_bytes());
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
        let next_raw = u64::from_le_bytes(reader.take_array::<8>()?);
        if next_raw == 0 {
            return Err(AnchorSnapshotCodecError::ZeroSequence);
        }
        let next_sequence = ParagraphAnchorSequence(next_raw);
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
        let mut greatest_live = None;

        for _ in 0..paragraph_count {
            let sequence_raw = u64::from_le_bytes(reader.take_array::<8>()?);
            if sequence_raw == 0 {
                return Err(AnchorSnapshotCodecError::ZeroSequence);
            }
            let sequence = ParagraphAnchorSequence(sequence_raw);
            if !live_sequences.insert(sequence) {
                return Err(AnchorSnapshotCodecError::DuplicateSequence(sequence));
            }
            greatest_live = Some(greatest_live.map_or(sequence, |current: ParagraphAnchorSequence| {
                current.max(sequence)
            }));

            let semantic_len = u32::from_le_bytes(reader.take_array::<4>()?);
            if semantic_len > limits.max_paragraph_bytes {
                return Err(AnchorSnapshotCodecError::ParagraphBytesLimitExceeded {
                    actual: semantic_len,
                    limit: limits.max_paragraph_bytes,
                });
            }
            total_text_bytes = total_text_bytes
                .checked_add(u64::from(semantic_len))
                .ok_or(AnchorSnapshotCodecError::LengthOverflow)?;
            if total_text_bytes > limits.max_total_text_bytes {
                return Err(AnchorSnapshotCodecError::TotalTextBytesLimitExceeded {
                    actual: total_text_bytes,
                    limit: limits.max_total_text_bytes,
                });
            }
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
        if let Some(greatest_live) = greatest_live
            && next_sequence <= greatest_live
        {
            return Err(AnchorSnapshotCodecError::NextSequenceNotAfterLiveAnchors {
                next: next_sequence,
                greatest_live,
            });
        }

        let snapshot = Self {
            lineage,
            next_sequence,
            paragraphs,
        };
        validate_snapshot(&snapshot, limits)?;
        Ok(snapshot)
    }
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
    let mut greatest_live = None;
    let mut total_text_bytes = 0_u64;
    for record in &snapshot.paragraphs {
        let sequence = record.id.sequence;
        if sequence.0 == 0 || snapshot.next_sequence.0 == 0 {
            return Err(AnchorSnapshotCodecError::ZeroSequence);
        }
        if record.id.lineage != snapshot.lineage {
            return Err(AnchorSnapshotCodecError::LengthOverflow);
        }
        if !live_sequences.insert(sequence) {
            return Err(AnchorSnapshotCodecError::DuplicateSequence(sequence));
        }
        greatest_live = Some(greatest_live.map_or(sequence, |current: ParagraphAnchorSequence| {
            current.max(sequence)
        }));

        let semantic_len = u32::try_from(record.semantic_text.len())
            .map_err(|_| AnchorSnapshotCodecError::LengthOverflow)?;
        if semantic_len > limits.max_paragraph_bytes {
            return Err(AnchorSnapshotCodecError::ParagraphBytesLimitExceeded {
                actual: semantic_len,
                limit: limits.max_paragraph_bytes,
            });
        }
        total_text_bytes = total_text_bytes
            .checked_add(u64::from(semantic_len))
            .ok_or(AnchorSnapshotCodecError::LengthOverflow)?;
        if total_text_bytes > limits.max_total_text_bytes {
            return Err(AnchorSnapshotCodecError::TotalTextBytesLimitExceeded {
                actual: total_text_bytes,
                limit: limits.max_total_text_bytes,
            });
        }
    }

    if let Some(greatest_live) = greatest_live
        && snapshot.next_sequence <= greatest_live
    {
        return Err(AnchorSnapshotCodecError::NextSequenceNotAfterLiveAnchors {
            next: snapshot.next_sequence,
            greatest_live,
        });
    }
    Ok(())
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
        let slice = self.take_slice(N)?;
        slice
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

#[cfg(test)]
mod tests {
    use super::*;

    const LINEAGE: DocumentLineageId = DocumentLineageId::from_bytes([0x11; 16]);
    const OTHER_LINEAGE: DocumentLineageId = DocumentLineageId::from_bytes([0x22; 16]);
    const LIMITS: AnchorSnapshotLimits = AnchorSnapshotLimits::new(64, 1024, 16 * 1024);

    fn strings(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).to_owned()).collect()
    }

    fn sequences(table: &ParagraphAnchorTable) -> Vec<u64> {
        table
            .paragraphs()
            .iter()
            .map(|record| record.id().sequence().get())
            .collect()
    }

    #[test]
    fn structural_policy_is_product_owned_and_round_trip_stable() {
        let mut table = ParagraphAnchorTable::from_projection(LINEAGE, strings(&["P0", "P1", "P2"]))
            .expect("initial projection must fit anchor sequence");
        assert_eq!(sequences(&table), vec![1, 2, 3]);

        let (left, right) = table
            .split_paragraph(0, "P0-left".into(), "P0-right".into())
            .expect("qualified split must be representable");
        assert_eq!(left.sequence().get(), 1);
        assert_eq!(right.sequence().get(), 4);
        assert_eq!(sequences(&table), vec![1, 4, 2, 3]);

        let merged = table
            .merge_with_next(0, "P0".into())
            .expect("qualified merge must be representable");
        assert_eq!(merged.surviving(), left);
        assert_eq!(merged.retired(), right);
        assert_eq!(sequences(&table), vec![1, 2, 3]);
        assert_eq!(
            table
                .paragraphs()
                .iter()
                .map(ParagraphAnchorRecord::semantic_text)
                .collect::<Vec<_>>(),
            vec!["P0", "P1", "P2"]
        );
    }

    #[test]
    fn ordinary_semantic_change_preserves_existing_anchor() {
        let mut table = ParagraphAnchorTable::from_projection(LINEAGE, strings(&["alpha", "beta"]))
            .expect("initial projection must fit anchor sequence");
        let before = table.paragraphs()[0].id();
        let after = table
            .replace_paragraph_text(0, "alpha edited".into())
            .expect("existing paragraph mutation must be valid");
        assert_eq!(before, after);
    }

    #[test]
    fn duplicate_text_is_not_used_as_identity_during_exact_rebind() {
        let table = ParagraphAnchorTable::from_projection(
            LINEAGE,
            strings(&["duplicate", "duplicate", "tail"]),
        )
        .expect("initial projection must fit anchor sequence");
        assert_ne!(table.paragraphs()[0].id(), table.paragraphs()[1].id());

        let encoded = table
            .snapshot()
            .encode(LIMITS)
            .expect("bounded snapshot must encode");
        let decoded = ParagraphAnchorSnapshot::decode(&encoded, LIMITS)
            .expect("bounded snapshot must decode");
        let rebound = decoded
            .rebind_exact_projection(LINEAGE, &strings(&["duplicate", "duplicate", "tail"]))
            .expect("exact known-lineage projection must rebind");
        assert_eq!(sequences(&rebound), vec![1, 2, 3]);
    }

    #[test]
    fn rebind_refuses_to_guess_after_semantic_or_lineage_change() {
        let table = ParagraphAnchorTable::from_projection(LINEAGE, strings(&["P0", "P1", "P2"]))
            .expect("initial projection must fit anchor sequence");
        let snapshot = table.snapshot();

        assert_eq!(
            snapshot.rebind_exact_projection(LINEAGE, &strings(&["P1", "P0", "P2"])),
            Err(AnchorRebindError::SemanticMismatch { index: 0 })
        );
        assert_eq!(
            snapshot.rebind_exact_projection(OTHER_LINEAGE, &strings(&["P0", "P1", "P2"])),
            Err(AnchorRebindError::LineageMismatch {
                expected: OTHER_LINEAGE,
                actual: LINEAGE,
            })
        );
    }

    #[test]
    fn retired_anchor_sequence_is_not_reused_after_snapshot_reload() {
        let mut table = ParagraphAnchorTable::from_projection(LINEAGE, strings(&["P0", "P1", "P2"]))
            .expect("initial projection must fit anchor sequence");
        let retired = table
            .insert_paragraph(1, "temporary".into())
            .expect("insert must mint a new anchor");
        assert_eq!(retired.sequence().get(), 4);
        assert_eq!(table.delete_paragraph(1), Ok(retired));

        let bytes = table
            .snapshot()
            .encode(LIMITS)
            .expect("bounded snapshot must encode");
        let snapshot = ParagraphAnchorSnapshot::decode(&bytes, LIMITS)
            .expect("bounded snapshot must decode");
        let mut rebound = snapshot
            .rebind_exact_projection(LINEAGE, &strings(&["P0", "P1", "P2"]))
            .expect("exact projection must rebind");
        let fresh = rebound
            .insert_paragraph(1, "new".into())
            .expect("post-reload insert must mint a new anchor");
        assert_eq!(fresh.sequence().get(), 5);
    }

    #[test]
    fn snapshot_decoder_enforces_bounds_and_complete_consumption() {
        let table = ParagraphAnchorTable::from_projection(LINEAGE, strings(&["P0", "P1"]))
            .expect("initial projection must fit anchor sequence");
        let bytes = table
            .snapshot()
            .encode(LIMITS)
            .expect("bounded snapshot must encode");

        let tiny_limits = AnchorSnapshotLimits::new(1, 1024, 16 * 1024);
        assert_eq!(
            ParagraphAnchorSnapshot::decode(&bytes, tiny_limits),
            Err(AnchorSnapshotCodecError::ParagraphLimitExceeded {
                actual: 2,
                limit: 1,
            })
        );

        let mut trailing = bytes.clone();
        trailing.push(0);
        assert_eq!(
            ParagraphAnchorSnapshot::decode(&trailing, LIMITS),
            Err(AnchorSnapshotCodecError::TrailingBytes)
        );
        assert_eq!(
            ParagraphAnchorSnapshot::decode(&bytes[..SNAPSHOT_HEADER_BYTES - 1], LIMITS),
            Err(AnchorSnapshotCodecError::Truncated)
        );
        assert_eq!(RECORD_HEADER_BYTES, 12);
    }
}
