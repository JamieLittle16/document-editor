#![doc = "UI-agnostic application orchestration."]

use document_engine_api::{DocumentEngine, EngineError};
use document_protocol::{DocumentTransaction, TextEdit, TextOffset};
use document_session::{DocumentSession, SessionObservation};

/// UI-safe projection of the current authoritative document state.
///
/// Toolkit code receives product-owned generation/revision numbers plus semantic text. It does
/// not receive engine objects, protocol transactions, or mutable session authority.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EditorSnapshot {
    authority_generation: u64,
    revision: u64,
    text: String,
}

impl EditorSnapshot {
    fn from_observation(observation: SessionObservation<String>) -> Self {
        let authority_generation = observation.authority_generation().get();
        let revision = observation.revision().get();
        let text = observation.into_value();
        Self {
            authority_generation,
            revision,
            text,
        }
    }

    #[must_use]
    pub const fn authority_generation(&self) -> u64 {
        self.authority_generation
    }

    #[must_use]
    pub const fn revision(&self) -> u64 {
        self.revision
    }

    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }

    #[must_use]
    pub fn into_text(self) -> String {
        self.text
    }
}

/// Product application core for one authoritative document session.
///
/// UI/toolkit code expresses user intent through this type. Protocol byte offsets and engine
/// revision checks remain below this boundary.
pub struct AppCore<E> {
    session: DocumentSession<E>,
}

impl<E: DocumentEngine> AppCore<E> {
    #[must_use]
    pub const fn new(engine: E) -> Self {
        Self {
            session: DocumentSession::new(engine),
        }
    }

    /// Opens a document from semantic text and returns the first current snapshot.
    pub fn open_text_document(&mut self, text: String) -> Result<EditorSnapshot, EngineError> {
        self.session.open_text_fixture(text)?;
        self.snapshot()
    }

    /// Returns a current, authority-validated semantic snapshot for presentation.
    pub fn snapshot(&self) -> Result<EditorSnapshot, EngineError> {
        self.session
            .semantic_text()
            .map(EditorSnapshot::from_observation)
    }

    /// Replaces the current semantic text as one user intent.
    ///
    /// The application layer computes the smallest single UTF-8-safe replacement window and
    /// sends that transaction through the authoritative session. A byte-identical replacement is
    /// a true no-op and does not advance revision.
    pub fn replace_document_text(
        &mut self,
        next_text: String,
    ) -> Result<EditorSnapshot, EngineError> {
        let current = self.session.semantic_text()?;
        if current.value() == &next_text {
            return Ok(EditorSnapshot::from_observation(current));
        }

        let expected_revision = current.revision();
        let (start, end, replacement) = replacement_window(current.value(), &next_text);
        let transaction = DocumentTransaction {
            expected_revision,
            edits: vec![TextEdit {
                start_utf8: text_offset(start)?,
                end_utf8: text_offset(end)?,
                replacement: replacement.to_owned(),
            }],
        };
        self.session.apply_transaction(transaction)?;
        self.snapshot()
    }
}

fn text_offset(index: usize) -> Result<TextOffset, EngineError> {
    u64::try_from(index)
        .map(TextOffset::new)
        .map_err(|_| EngineError::Internal(String::from("document text offset exceeds u64")))
}

fn replacement_window<'a>(current: &str, next: &'a str) -> (usize, usize, &'a str) {
    let prefix = common_prefix_bytes(current, next);
    let current_tail = &current[prefix..];
    let next_tail = &next[prefix..];
    let suffix = common_suffix_bytes(current_tail, next_tail);
    let current_end = current.len() - suffix;
    let next_end = next.len() - suffix;
    (prefix, current_end, &next[prefix..next_end])
}

fn common_prefix_bytes(left: &str, right: &str) -> usize {
    let mut bytes = 0;
    for (left_char, right_char) in left.chars().zip(right.chars()) {
        if left_char != right_char {
            break;
        }
        bytes += left_char.len_utf8();
    }
    bytes
}

fn common_suffix_bytes(left: &str, right: &str) -> usize {
    let mut bytes = 0;
    for (left_char, right_char) in left.chars().rev().zip(right.chars().rev()) {
        if left_char != right_char {
            break;
        }
        bytes += left_char.len_utf8();
    }
    bytes
}

#[cfg(test)]
mod tests {
    use document_engine_api::{DocumentEngine, SemanticObservation};
    use document_protocol::{
        DocumentCapability, DocumentRevision, EngineCapabilities, ProtocolError, ProtocolVersion,
        TransactionApplied, TransactionLimits,
    };

    use super::*;

    const TEST_LIMITS: TransactionLimits = TransactionLimits::new(16, 1024 * 1024, 2 * 1024 * 1024);

    #[derive(Default)]
    struct TestEngine {
        text: Option<String>,
        revision: DocumentRevision,
    }

    impl DocumentEngine for TestEngine {
        fn capabilities(&self) -> EngineCapabilities {
            EngineCapabilities {
                protocol: ProtocolVersion::V0,
                capabilities: vec![
                    DocumentCapability::Read,
                    DocumentCapability::EditText,
                    DocumentCapability::SemanticSnapshot,
                ],
            }
        }

        fn revision(&self) -> Result<DocumentRevision, EngineError> {
            self.text.as_ref().ok_or(EngineError::NotOpen)?;
            Ok(self.revision)
        }

        fn open_text_fixture(&mut self, text: String) -> Result<DocumentRevision, EngineError> {
            self.text = Some(text);
            self.revision = DocumentRevision::INITIAL;
            Ok(self.revision)
        }

        fn semantic_text(&self) -> Result<SemanticObservation<String>, EngineError> {
            let text = self.text.clone().ok_or(EngineError::NotOpen)?;
            Ok(SemanticObservation::new(self.revision, text))
        }

        fn apply_transaction(
            &mut self,
            transaction: DocumentTransaction,
        ) -> Result<TransactionApplied, EngineError> {
            if transaction.expected_revision != self.revision {
                return Err(ProtocolError::RevisionConflict {
                    expected: transaction.expected_revision,
                    actual: self.revision,
                }
                .into());
            }

            let text = self.text.as_mut().ok_or(EngineError::NotOpen)?;
            transaction.validate_against(text, TEST_LIMITS)?;
            for edit in transaction.edits.into_iter().rev() {
                let range = edit.byte_range(text)?;
                text.replace_range(range, &edit.replacement);
            }

            let previous_revision = self.revision;
            self.revision = self.revision.next();
            Ok(TransactionApplied {
                previous_revision,
                new_revision: self.revision,
            })
        }
    }

    #[test]
    fn open_and_replace_are_exposed_as_application_intent() {
        let mut app = AppCore::new(TestEngine::default());
        let opened = app.open_text_document(String::from("hello world")).unwrap();
        assert_eq!(opened.authority_generation(), 1);
        assert_eq!(opened.revision(), 0);
        assert_eq!(opened.text(), "hello world");

        let edited = app
            .replace_document_text(String::from("hello editor"))
            .unwrap();
        assert_eq!(edited.authority_generation(), 1);
        assert_eq!(edited.revision(), 1);
        assert_eq!(edited.text(), "hello editor");
    }

    #[test]
    fn identical_ui_text_is_a_true_no_op() {
        let mut app = AppCore::new(TestEngine::default());
        app.open_text_document(String::from("same")).unwrap();

        let unchanged = app.replace_document_text(String::from("same")).unwrap();

        assert_eq!(unchanged.revision(), 0);
        assert_eq!(unchanged.text(), "same");
    }

    #[test]
    fn reopening_advances_authority_even_when_engine_revision_restarts() {
        let mut app = AppCore::new(TestEngine::default());
        let first = app.open_text_document(String::from("first")).unwrap();
        let second = app.open_text_document(String::from("second")).unwrap();

        assert_eq!(first.revision(), 0);
        assert_eq!(second.revision(), 0);
        assert_eq!(first.authority_generation(), 1);
        assert_eq!(second.authority_generation(), 2);
    }

    #[test]
    fn replacement_window_preserves_utf8_boundaries() {
        let current = "A café and tea ☕";
        let next = "A caffè and tea ☕";
        let (start, end, replacement) = replacement_window(current, next);

        assert!(current.is_char_boundary(start));
        assert!(current.is_char_boundary(end));
        assert!(next.is_char_boundary(start));
        assert_eq!(&current[..start], &next[..start]);
        assert_eq!(replacement, "fè");

        let mut rebuilt = current.to_owned();
        rebuilt.replace_range(start..end, replacement);
        assert_eq!(rebuilt, next);
    }

    #[test]
    fn replacement_window_handles_insert_and_delete_without_crossing_prefix() {
        assert_eq!(replacement_window("abc", "abXYZc"), (2, 2, "XYZ"));
        assert_eq!(replacement_window("abXYZc", "abc"), (2, 5, ""));
    }
}
