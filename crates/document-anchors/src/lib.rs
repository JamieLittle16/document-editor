#![doc = "Product-owned durable paragraph anchors and conservative reconciliation artifacts."]

use std::fmt;

mod snapshot;

pub use snapshot::{
    AnchorRebindError, AnchorSnapshotCodecError, AnchorSnapshotLimits, SNAPSHOT_FORMAT_VERSION,
};

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
    const fn new(id: ParagraphAnchorId, semantic_text: String) -> Self {
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
    ParagraphOutOfBounds {
        index: usize,
        paragraph_count: usize,
    },
    NoFollowingParagraph {
        index: usize,
        paragraph_count: usize,
    },
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
        self.paragraphs
            .insert(index + 1, ParagraphAnchorRecord::new(right_id, right_text));
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
        Ok(ParagraphMergeApplied { surviving, retired })
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

/// Bounded, versioned durable artifact for product-owned paragraph anchors.
///
/// Semantic text is stored only to verify an exact known-lineage rebind. It is never used as an
/// identity key or searched for candidate matches. If the projection differs, the operation is
/// unresolved and a richer structural/history reconciliation path must decide what happened.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParagraphAnchorSnapshot {
    pub(crate) lineage: DocumentLineageId,
    pub(crate) next_sequence: ParagraphAnchorSequence,
    pub(crate) paragraphs: Vec<ParagraphAnchorRecord>,
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
}
