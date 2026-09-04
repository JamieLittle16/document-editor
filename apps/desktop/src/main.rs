use document_engine_mock::MockDocumentEngine;
use document_protocol::{DocumentRevision, DocumentTransaction, TextEdit, TextOffset};
use document_session::DocumentSession;

fn main() {
    let mut session = DocumentSession::new(MockDocumentEngine::default());
    session
        .open_text_fixture(String::from("Document editor architecture spike"))
        .expect("open fixture");
    session
        .apply_transaction(DocumentTransaction {
            expected_revision: DocumentRevision::INITIAL,
            edits: vec![TextEdit {
                start_utf8: TextOffset::new(0),
                end_utf8: TextOffset::new(8),
                replacement: String::from("Modern document"),
            }],
        })
        .expect("apply transaction");

    println!("{}", session.semantic_text().expect("semantic text"));
}
