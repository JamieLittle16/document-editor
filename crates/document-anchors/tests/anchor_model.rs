use document_anchors::{
    AnchorRebindError, AnchorSnapshotCodecError, AnchorSnapshotLimits, DocumentLineageId,
    ParagraphAnchorRecord, ParagraphAnchorSnapshot, ParagraphAnchorTable,
};

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

fn semantic_text(table: &ParagraphAnchorTable) -> Vec<&str> {
    table
        .paragraphs()
        .iter()
        .map(ParagraphAnchorRecord::semantic_text)
        .collect()
}

#[test]
fn split_merge_policy_is_product_owned_and_round_trip_stable() {
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
    assert_eq!(semantic_text(&table), vec!["P0", "P1", "P2"]);
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
    let decoded =
        ParagraphAnchorSnapshot::decode(&encoded, LIMITS).expect("bounded snapshot must decode");
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
    let snapshot =
        ParagraphAnchorSnapshot::decode(&bytes, LIMITS).expect("bounded snapshot must decode");
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
        ParagraphAnchorSnapshot::decode(&bytes[..39], LIMITS),
        Err(AnchorSnapshotCodecError::Truncated)
    );
}
