use app_core::reconciliation::{
    HistoryLineageId, LogicalAnchorAllocator, ReconciliationBasis, ReconciliationCandidate,
    ReconciliationOutcome, reconcile_anchor,
};
use document_engine_mock::MockDocumentEngine;
use document_protocol::{DocumentRevision, DocumentTransaction, TextEdit, TextOffset};
use document_session::DocumentSession;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ParagraphLocator(u64);

fn append(expected_revision: DocumentRevision, offset: u64, text: &str) -> DocumentTransaction {
    DocumentTransaction {
        expected_revision,
        edits: vec![TextEdit {
            start_utf8: TextOffset::new(offset),
            end_utf8: TextOffset::new(offset),
            replacement: String::from(text),
        }],
    }
}

#[test]
fn same_authority_engine_continuity_is_valid_positive_evidence_across_revision_change() {
    let mut session = DocumentSession::new(MockDocumentEngine::default());
    session.open_text_fixture(String::from("abc")).unwrap();

    let mut allocator = LogicalAnchorAllocator::new(HistoryLineageId::new(7));
    let anchor = allocator.mint(()).unwrap();
    let initial_authority = session.current_authority_stamp().unwrap();
    let prior = anchor.bind(initial_authority, ParagraphLocator(0));

    session
        .apply_transaction(append(DocumentRevision::INITIAL, 3, "!"))
        .unwrap();
    let current_authority = session.current_authority_stamp().unwrap();
    assert_eq!(
        current_authority.authority_generation(),
        initial_authority.authority_generation()
    );
    assert_ne!(current_authority.revision(), initial_authority.revision());

    let candidate = ReconciliationCandidate::new(current_authority, ParagraphLocator(0))
        .with_same_authority_engine_continuity(&prior)
        .unwrap();
    let outcome = reconcile_anchor(anchor.id(), current_authority, &[candidate]).unwrap();

    let ReconciliationOutcome::Rebound { binding, basis } = outcome else {
        panic!("same-authority object continuity should be positive reconciliation evidence");
    };
    assert_eq!(binding.anchor(), anchor.id());
    assert_eq!(*binding.target(), ParagraphLocator(0));
    assert_eq!(binding.authority(), current_authority);
    assert_eq!(basis, ReconciliationBasis::SameAuthorityEngineContinuity);
}

#[test]
fn explicit_product_lineage_beats_incidental_structural_semantic_similarity() {
    let mut session = DocumentSession::new(MockDocumentEngine::default());
    session
        .open_text_fixture(String::from("left\nsame\nright"))
        .unwrap();

    let mut allocator = LogicalAnchorAllocator::new(HistoryLineageId::new(8));
    let anchor = allocator.mint(()).unwrap();
    let applied = session
        .apply_transaction(append(DocumentRevision::INITIAL, 15, "!"))
        .unwrap();
    let authority = session.current_authority_stamp().unwrap();

    let candidates = [
        ReconciliationCandidate::new(authority, ParagraphLocator(0))
            .with_structural_neighbourhood_match()
            .with_semantic_equivalence(),
        ReconciliationCandidate::new(authority, ParagraphLocator(1))
            .with_product_lineage(applied.sequence()),
    ];

    let outcome = reconcile_anchor(anchor.id(), authority, &candidates).unwrap();
    let ReconciliationOutcome::Rebound { binding, basis } = outcome else {
        panic!("explicit product lineage should determine the target");
    };

    assert_eq!(*binding.target(), ParagraphLocator(1));
    assert_eq!(basis, ReconciliationBasis::ProductLineage(applied.sequence()));
}

#[test]
fn conflicting_product_lineage_candidates_are_never_guessed_between() {
    let mut session = DocumentSession::new(MockDocumentEngine::default());
    session.open_text_fixture(String::from("abc")).unwrap();
    let applied = session
        .apply_transaction(append(DocumentRevision::INITIAL, 3, "!"))
        .unwrap();
    let authority = session.current_authority_stamp().unwrap();

    let mut allocator = LogicalAnchorAllocator::new(HistoryLineageId::new(9));
    let anchor = allocator.mint(()).unwrap();
    let candidates = [
        ReconciliationCandidate::new(authority, ParagraphLocator(0))
            .with_product_lineage(applied.sequence()),
        ReconciliationCandidate::new(authority, ParagraphLocator(1))
            .with_product_lineage(applied.sequence()),
    ];

    assert_eq!(
        reconcile_anchor(anchor.id(), authority, &candidates).unwrap(),
        ReconciliationOutcome::Ambiguous
    );
}
