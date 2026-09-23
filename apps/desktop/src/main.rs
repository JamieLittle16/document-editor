use app_core::AppCore;
use document_engine_mock::MockDocumentEngine;

fn main() {
    let mut app = AppCore::new(MockDocumentEngine::default());
    app.open_text_document(String::from("Document editor architecture spike"))
        .expect("open fixture");

    let edited = app
        .replace_document_text("Modern document architecture spike")
        .expect("replace document text");

    println!("{}", edited.text());
}
