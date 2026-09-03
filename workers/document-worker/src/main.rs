use document_engine_api::DocumentEngine;
use document_engine_mock::MockDocumentEngine;

fn main() {
    let engine = MockDocumentEngine::default();
    let capabilities = engine.capabilities();
    println!("document-worker protocol {}.{}", capabilities.protocol.major, capabilities.protocol.minor);
}
