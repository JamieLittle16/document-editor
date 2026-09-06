//! Product-owned logical anchor identity and conservative reconciliation.
//!
//! Durable anchor identity is intentionally separate from live engine/session bindings. A binding
//! may be replaced after a structural edit, save/reload, worker restart or checkpoint recovery
//! while the logical anchor remains unchanged.

use std::fmt;

use document_session::{
    AcceptedOperationSequence, AuthorityGeneration, SessionAuthorityStamp,
};

/// Product-owned history lineage namespace.
///
/// This is not an engine document ID, file-format ID or authority generation. The history layer is
/// responsible for creating and persisting the value and may later define explicit fork/save-as
/// policy around it.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct HistoryLineageId(u128);

impl HistoryLineageId {
    #[must_use]
    pub const fn new(value: u128) -> Self {
        Self(value)
    }

    #[must_use]
    pub const fn get(self) -> u128 {
        self.0
    }
}

/// Durable product-owned identity for one logical anchor inside a history lineage.
///
/// The local sequence is meaningful only together with `HistoryLineageId`. It must never be
/// derived from a Writer/UNO object, OOXML identifier, text hash or byte offset.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct LogicalAnchorId {
    lineage: HistoryLineageId,
    local_sequence: u64,
}

impl LogicalAnchorId {
    const fn new(lineage: HistoryLineageId, local_sequence: u64) -> Self {
        Self {
            lineage,
            local_sequence,
        }
    }

    #[must_use]
    pub const fn lineage(self) -> HistoryLineageId {
        self.lineage
    }

    #[must_use]
    pub const fn local_sequence(self) -> u64 {
        self.local_sequence
    }
}

/// Monotonic allocator for logical anchors inside one history lineage.
///
/// The `last_issued` cursor is product-owned persistence metadata. Restoring it is sufficient to
/// prevent ID reuse after a reload without depending on engine identity.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LogicalAnchorAllocator {
    lineage: HistoryLineageId,
    last_issued: u64,
}

impl LogicalAnchorAllocator {
    #[must_use]
    pub const fn new(lineage: HistoryLineageId) -> Self {
        Self {
            lineage,
            last_issued: 0,
        }
    }

    #[must_use]
    pub const fn resume(lineage: HistoryLineageId, last_issued: u64) -> Self {
        Self {
            lineage,
            last_issued,
        }
    }

    #[must_use]
    pub const fn lineage(self) -> HistoryLineageId {
        self.lineage
    }

    #[must_use]
    pub const fn last_issued(self) -> u64 {
        self.last_issued
    }

    pub fn mint<H>(&mut self, hint: H) -> Result<DurableLogicalAnchor<H>, AnchorAllocationError> {
        let next = self
            .last_issued
            .checked_add(1)
            .ok_or(AnchorAllocationError::SequenceExhausted)?;
        self.last_issued = next;
        Ok(DurableLogicalAnchor {
            id: LogicalAnchorId::new(self.lineage, next),
            hint,
        })
    }
}

/// Logical-anchor allocation cannot continue without re-namespacing the history lineage.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AnchorAllocationError {
    SequenceExhausted,
}

impl fmt::Display for AnchorAllocationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SequenceExhausted => formatter.write_str("logical anchor sequence exhausted"),
        }
    }
}

impl std::error::Error for AnchorAllocationError {}

/// Persistable product anchor plus caller-defined reconciliation hint.
///
/// `H` is deliberately product-owned evidence rather than a prescribed engine locator. It may be
/// a normalized structural/semantic hint appropriate to a future projection, but it is not part of
/// the anchor's identity. This record contains no authority generation, revision, UNO reference or
/// engine probe token.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DurableLogicalAnchor<H> {
    id: LogicalAnchorId,
    hint: H,
}

impl<H> DurableLogicalAnchor<H> {
    #[must_use]
    pub const fn id(&self) -> LogicalAnchorId {
        self.id
    }

    #[must_use]
    pub const fn hint(&self) -> &H {
        &self.hint
    }

    #[must_use]
    pub fn into_hint(self) -> H {
        self.hint
    }

    #[must_use]
    pub fn map_hint<U>(self, map: impl FnOnce(H) -> U) -> DurableLogicalAnchor<U> {
        DurableLogicalAnchor {
            id: self.id,
            hint: map(self.hint),
        }
    }

    /// Establish an initial live binding from an already-authoritative product projection.
    #[must_use]
    pub fn bind<T>(&self, authority: SessionAuthorityStamp, target: T) -> LiveAnchorBinding<T> {
        LiveAnchorBinding::new(self.id, authority, target)
    }
}

/// Ephemeral binding of a durable anchor to one target in one exact live authority/revision.
///
/// The target is intentionally generic because the permanent semantic-structure locator schema is
/// not frozen in R0A. This value is runtime state, not the durable anchor artifact.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LiveAnchorBinding<T> {
    anchor: LogicalAnchorId,
    authority: SessionAuthorityStamp,
    target: T,
}

impl<T> LiveAnchorBinding<T> {
    const fn new(anchor: LogicalAnchorId, authority: SessionAuthorityStamp, target: T) -> Self {
        Self {
            anchor,
            authority,
            target,
        }
    }

    #[must_use]
    pub const fn anchor(&self) -> LogicalAnchorId {
        self.anchor
    }

    #[must_use]
    pub const fn authority(&self) -> SessionAuthorityStamp {
        self.authority
    }

    #[must_use]
    pub const fn target(&self) -> &T {
        &self.target
    }
}

/// Conservative evidence channels for one candidate target.
///
/// No individual weak channel becomes identity. Product lineage and same-live-engine-object
/// continuity are strong positive evidence. Structural + semantic agreement is a fallback only
/// when it uniquely identifies a candidate. Semantic equality alone is never sufficient.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ReconciliationEvidence {
    product_lineage: Option<AcceptedOperationSequence>,
    same_authority_engine_continuity: bool,
    structural_neighbourhood_match: bool,
    semantic_equivalence: bool,
}

impl ReconciliationEvidence {
    #[must_use]
    pub const fn product_lineage(self) -> Option<AcceptedOperationSequence> {
        self.product_lineage
    }

    #[must_use]
    pub const fn same_authority_engine_continuity(self) -> bool {
        self.same_authority_engine_continuity
    }

    #[must_use]
    pub const fn structural_neighbourhood_match(self) -> bool {
        self.structural_neighbourhood_match
    }

    #[must_use]
    pub const fn semantic_equivalence(self) -> bool {
        self.semantic_equivalence
    }
}

/// One possible target for a durable anchor in a new semantic observation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReconciliationCandidate<T> {
    authority: SessionAuthorityStamp,
    target: T,
    evidence: ReconciliationEvidence,
}

impl<T> ReconciliationCandidate<T> {
    #[must_use]
    pub const fn new(authority: SessionAuthorityStamp, target: T) -> Self {
        Self {
            authority,
            target,
            evidence: ReconciliationEvidence {
                product_lineage: None,
                same_authority_engine_continuity: false,
                structural_neighbourhood_match: false,
                semantic_equivalence: false,
            },
        }
    }

    /// Record an explicit product-owned operation mapping from the prior logical target to this
    /// candidate. The sequence is audit evidence; the anchor ID remains the durable identity.
    #[must_use]
    pub const fn with_product_lineage(mut self, operation: AcceptedOperationSequence) -> Self {
        self.evidence.product_lineage = Some(operation);
        self
    }

    #[must_use]
    pub const fn with_structural_neighbourhood_match(mut self) -> Self {
        self.evidence.structural_neighbourhood_match = true;
        self
    }

    #[must_use]
    pub const fn with_semantic_equivalence(mut self) -> Self {
        self.evidence.semantic_equivalence = true;
        self
    }

    /// Record same-live-engine-object continuity without persisting the native token itself.
    ///
    /// This channel is forbidden across authority replacement. Callers may derive the boolean from
    /// version-pinned engine evidence, but only the product-neutral fact enters reconciliation.
    pub fn with_same_authority_engine_continuity<U>(
        mut self,
        prior: &LiveAnchorBinding<U>,
    ) -> Result<Self, ReconciliationEvidenceError> {
        let source = prior.authority().authority_generation();
        let candidate = self.authority.authority_generation();
        if source != candidate {
            return Err(ReconciliationEvidenceError::EngineContinuityCrossesAuthority {
                source,
                candidate,
            });
        }
        self.evidence.same_authority_engine_continuity = true;
        Ok(self)
    }

    #[must_use]
    pub const fn authority(&self) -> SessionAuthorityStamp {
        self.authority
    }

    #[must_use]
    pub const fn target(&self) -> &T {
        &self.target
    }

    #[must_use]
    pub const fn evidence(&self) -> ReconciliationEvidence {
        self.evidence
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReconciliationEvidenceError {
    EngineContinuityCrossesAuthority {
        source: AuthorityGeneration,
        candidate: AuthorityGeneration,
    },
}

impl fmt::Display for ReconciliationEvidenceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EngineContinuityCrossesAuthority { source, candidate } => write!(
                formatter,
                "engine-object continuity cannot cross authority generations {source} -> {candidate}"
            ),
        }
    }
}

impl std::error::Error for ReconciliationEvidenceError {}

/// Positive basis used to establish a replacement live binding.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReconciliationBasis {
    ProductLineage(AcceptedOperationSequence),
    SameAuthorityEngineContinuity,
    UniqueStructuralAndSemanticMatch,
}

/// Conservative result of reconciling one durable anchor against one current semantic snapshot.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ReconciliationOutcome<T> {
    Rebound {
        binding: LiveAnchorBinding<T>,
        basis: ReconciliationBasis,
    },
    Ambiguous,
    Unresolved,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReconciliationError {
    CandidateAuthorityMismatch {
        expected: SessionAuthorityStamp,
        actual: SessionAuthorityStamp,
    },
}

impl fmt::Display for ReconciliationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CandidateAuthorityMismatch { expected, actual } => write!(
                formatter,
                "reconciliation candidate authority {:?}/{} does not match target {:?}/{}",
                actual.authority_generation(),
                actual.revision(),
                expected.authority_generation(),
                expected.revision()
            ),
        }
    }
}

impl std::error::Error for ReconciliationError {}

/// Reconcile one durable anchor into one current authority snapshot.
///
/// The decision is intentionally lexicographic and conservative rather than score-based:
///
/// 1. a unique product-owned lineage mapping wins, unless it conflicts with a different
///    same-authority engine-continuity candidate;
/// 2. otherwise a unique same-authority engine-continuity candidate wins;
/// 3. otherwise a unique candidate matching both structural neighbourhood and semantics wins;
/// 4. multiple plausible candidates are ambiguous; insufficient evidence is unresolved.
pub fn reconcile_anchor<T: Clone>(
    anchor: LogicalAnchorId,
    target_authority: SessionAuthorityStamp,
    candidates: &[ReconciliationCandidate<T>],
) -> Result<ReconciliationOutcome<T>, ReconciliationError> {
    for candidate in candidates {
        if candidate.authority() != target_authority {
            return Err(ReconciliationError::CandidateAuthorityMismatch {
                expected: target_authority,
                actual: candidate.authority(),
            });
        }
    }

    let product_lineage = candidates
        .iter()
        .enumerate()
        .filter(|(_, candidate)| candidate.evidence().product_lineage().is_some())
        .collect::<Vec<_>>();
    if product_lineage.len() > 1 {
        return Ok(ReconciliationOutcome::Ambiguous);
    }
    if let Some((index, candidate)) = product_lineage.first().copied() {
        let conflicting_engine_evidence = candidates.iter().enumerate().any(|(other_index, other)| {
            other_index != index && other.evidence().same_authority_engine_continuity()
        });
        if conflicting_engine_evidence {
            return Ok(ReconciliationOutcome::Ambiguous);
        }
        let operation = candidate
            .evidence()
            .product_lineage()
            .expect("filtered product-lineage candidate must carry operation evidence");
        return Ok(ReconciliationOutcome::Rebound {
            binding: LiveAnchorBinding::new(anchor, target_authority, candidate.target().clone()),
            basis: ReconciliationBasis::ProductLineage(operation),
        });
    }

    let engine_continuity = candidates
        .iter()
        .filter(|candidate| candidate.evidence().same_authority_engine_continuity())
        .collect::<Vec<_>>();
    if engine_continuity.len() > 1 {
        return Ok(ReconciliationOutcome::Ambiguous);
    }
    if let Some(candidate) = engine_continuity.first().copied() {
        return Ok(ReconciliationOutcome::Rebound {
            binding: LiveAnchorBinding::new(anchor, target_authority, candidate.target().clone()),
            basis: ReconciliationBasis::SameAuthorityEngineContinuity,
        });
    }

    let structural_semantic = candidates
        .iter()
        .filter(|candidate| {
            let evidence = candidate.evidence();
            evidence.structural_neighbourhood_match() && evidence.semantic_equivalence()
        })
        .collect::<Vec<_>>();
    if structural_semantic.len() > 1 {
        return Ok(ReconciliationOutcome::Ambiguous);
    }
    if let Some(candidate) = structural_semantic.first().copied() {
        return Ok(ReconciliationOutcome::Rebound {
            binding: LiveAnchorBinding::new(anchor, target_authority, candidate.target().clone()),
            basis: ReconciliationBasis::UniqueStructuralAndSemanticMatch,
        });
    }

    let weak_plausible_count = candidates
        .iter()
        .filter(|candidate| {
            let evidence = candidate.evidence();
            evidence.structural_neighbourhood_match() || evidence.semantic_equivalence()
        })
        .count();
    if weak_plausible_count > 1 {
        Ok(ReconciliationOutcome::Ambiguous)
    } else {
        Ok(ReconciliationOutcome::Unresolved)
    }
}

#[cfg(test)]
mod tests {
    use document_engine_mock::MockDocumentEngine;
    use document_session::DocumentSession;

    use super::*;

    #[derive(Clone, Debug, Eq, PartialEq)]
    struct ParagraphHint {
        text: String,
        previous_text: Option<String>,
        next_text: Option<String>,
    }

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    struct ParagraphLocator(u64);

    fn lineage() -> HistoryLineageId {
        HistoryLineageId::new(0x0ff1_ce00_0000_0000_0000_0000_0000_0001)
    }

    fn anchor_fixture() -> DurableLogicalAnchor<ParagraphHint> {
        let mut allocator = LogicalAnchorAllocator::new(lineage());
        allocator
            .mint(ParagraphHint {
                text: String::from("same"),
                previous_text: Some(String::from("left")),
                next_text: Some(String::from("right")),
            })
            .unwrap()
    }

    #[test]
    fn anchor_allocator_is_monotonic_and_resumable_without_engine_identity() {
        let mut allocator = LogicalAnchorAllocator::new(lineage());
        let first = allocator.mint(()).unwrap();
        let second = allocator.mint(()).unwrap();
        let persisted_cursor = allocator.last_issued();
        let mut resumed = LogicalAnchorAllocator::resume(lineage(), persisted_cursor);
        let third = resumed.mint(()).unwrap();

        assert_eq!(first.id().local_sequence(), 1);
        assert_eq!(second.id().local_sequence(), 2);
        assert_eq!(third.id().local_sequence(), 3);
        assert_eq!(first.id().lineage(), lineage());
        assert_eq!(third.id().lineage(), lineage());
    }

    #[test]
    fn duplicate_semantics_are_ambiguous_without_structural_or_lineage_evidence() {
        let mut session = DocumentSession::new(MockDocumentEngine::default());
        session.open_text_fixture(String::from("same\nsame")).unwrap();
        let authority = session.current_authority_stamp().unwrap();
        let anchor = anchor_fixture();

        let candidates = [
            ReconciliationCandidate::new(authority, ParagraphLocator(0))
                .with_semantic_equivalence(),
            ReconciliationCandidate::new(authority, ParagraphLocator(1))
                .with_semantic_equivalence(),
        ];

        assert_eq!(
            reconcile_anchor(anchor.id(), authority, &candidates).unwrap(),
            ReconciliationOutcome::Ambiguous
        );
    }

    #[test]
    fn unique_structural_and_semantic_evidence_rebinds_without_engine_identity() {
        let mut session = DocumentSession::new(MockDocumentEngine::default());
        session
            .open_text_fixture(String::from("left\nsame\nright"))
            .unwrap();
        let authority = session.current_authority_stamp().unwrap();
        let anchor = anchor_fixture();

        let candidates = [
            ReconciliationCandidate::new(authority, ParagraphLocator(0)),
            ReconciliationCandidate::new(authority, ParagraphLocator(1))
                .with_structural_neighbourhood_match()
                .with_semantic_equivalence(),
            ReconciliationCandidate::new(authority, ParagraphLocator(2)),
        ];

        let ReconciliationOutcome::Rebound { binding, basis } =
            reconcile_anchor(anchor.id(), authority, &candidates).unwrap()
        else {
            panic!("unique structural + semantic evidence should rebind");
        };
        assert_eq!(binding.anchor(), anchor.id());
        assert_eq!(binding.authority(), authority);
        assert_eq!(*binding.target(), ParagraphLocator(1));
        assert_eq!(basis, ReconciliationBasis::UniqueStructuralAndSemanticMatch);
    }

    #[test]
    fn same_engine_object_is_positive_only_inside_one_authority_generation() {
        let mut session = DocumentSession::new(MockDocumentEngine::default());
        session.open_text_fixture(String::from("abc")).unwrap();
        let anchor = anchor_fixture();
        let initial_authority = session.current_authority_stamp().unwrap();
        let initial = anchor.bind(initial_authority, ParagraphLocator(0));

        session.open_text_fixture(String::from("abc")).unwrap();
        let replacement_authority = session.current_authority_stamp().unwrap();
        let candidate = ReconciliationCandidate::new(replacement_authority, ParagraphLocator(0));

        assert!(matches!(
            candidate.with_same_authority_engine_continuity(&initial),
            Err(ReconciliationEvidenceError::EngineContinuityCrossesAuthority { .. })
        ));
    }

    #[test]
    fn save_reload_rebinds_same_durable_anchor_under_fresh_authority() {
        let mut session = DocumentSession::new(MockDocumentEngine::default());
        session
            .open_text_fixture(String::from("left\nsame\nright"))
            .unwrap();
        let anchor = anchor_fixture();
        let first_authority = session.current_authority_stamp().unwrap();
        let first_binding = anchor.bind(first_authority, ParagraphLocator(1));

        let saved_anchor_artifact = anchor.clone();
        let saved_document_artifact = session.semantic_text().unwrap().into_value();
        session.open_text_fixture(saved_document_artifact).unwrap();
        let reloaded_authority = session.current_authority_stamp().unwrap();

        assert_ne!(
            first_binding.authority().authority_generation(),
            reloaded_authority.authority_generation()
        );
        let candidates = [
            ReconciliationCandidate::new(reloaded_authority, ParagraphLocator(0)),
            ReconciliationCandidate::new(reloaded_authority, ParagraphLocator(1))
                .with_structural_neighbourhood_match()
                .with_semantic_equivalence(),
            ReconciliationCandidate::new(reloaded_authority, ParagraphLocator(2)),
        ];
        let ReconciliationOutcome::Rebound { binding, .. } =
            reconcile_anchor(saved_anchor_artifact.id(), reloaded_authority, &candidates).unwrap()
        else {
            panic!("reload should rebind the durable anchor from product evidence");
        };

        assert_eq!(binding.anchor(), first_binding.anchor());
        assert_eq!(binding.authority(), reloaded_authority);
        assert_eq!(*binding.target(), ParagraphLocator(1));
        assert_eq!(saved_anchor_artifact.hint(), anchor.hint());
    }

    #[test]
    fn checkpoint_recovery_preserves_anchor_identity_but_replaces_live_binding_authority() {
        let mut session = DocumentSession::new(MockDocumentEngine::default());
        session
            .open_text_fixture(String::from("left\nsame\nright"))
            .unwrap();
        let anchor = anchor_fixture();
        let original_authority = session.current_authority_stamp().unwrap();
        let original_binding = anchor.bind(original_authority, ParagraphLocator(1));
        let observation = session.semantic_text().unwrap();
        let checkpoint = session.capture_recovery_checkpoint(&observation).unwrap();

        session.replace_engine_after_authority_loss(MockDocumentEngine::default());
        session
            .recover_text_fixture_from_checkpoint(&checkpoint)
            .unwrap();
        let recovered_authority = session.current_authority_stamp().unwrap();

        assert_ne!(
            original_binding.authority().authority_generation(),
            recovered_authority.authority_generation()
        );
        let candidates = [
            ReconciliationCandidate::new(recovered_authority, ParagraphLocator(0)),
            ReconciliationCandidate::new(recovered_authority, ParagraphLocator(1))
                .with_structural_neighbourhood_match()
                .with_semantic_equivalence(),
            ReconciliationCandidate::new(recovered_authority, ParagraphLocator(2)),
        ];
        let ReconciliationOutcome::Rebound { binding, .. } =
            reconcile_anchor(anchor.id(), recovered_authority, &candidates).unwrap()
        else {
            panic!("recovery should rebind the durable anchor under replacement authority");
        };

        assert_eq!(binding.anchor(), original_binding.anchor());
        assert_eq!(binding.authority(), recovered_authority);
        assert_eq!(*binding.target(), ParagraphLocator(1));
    }
}
