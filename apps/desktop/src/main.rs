use document_engine_mock::MockDocumentEngine;
use document_protocol::{DocumentRevision, DocumentTransaction, TextEdit};
use document_session::DocumentSession;

fn main() {
    let mut session = DocumentSession::new(MockDocumentEngine::default());
    session.open_text_fixture("Document editor architecture spike".into()).expect("open fixture");
    session
        .apply_transaction(DocumentTransaction {
            expected_revision: DocumentRevision::INITIAL,
            edits: vec![TextEdit {
                start_utf8: 0,
                end_utf8: 8,
                replacement: "Modern document".into(),
            }],
        })
        .expect("apply transaction");

    println!("{}", session.semantic_text().expect("semantic text"));
}
