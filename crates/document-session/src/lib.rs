#![doc = "Authority-, revision- and recovery-aware orchestration around a replaceable document engine."]

use std::fmt;

use document_engine_api::{DocumentEngine, EngineError, SemanticObservation};
use document_protocol::{DocumentRevision, DocumentTransaction, TransactionApplied};

/// One application-owned incarnation of the authoritative document binding.
///
/// A generation changes when this session successfully binds to a newly opened/reopened
/// authority. It is deliberately not a durable `DocumentId`: one logical document may survive
/// several authority generations as workers restart or engine bindings are replaced.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct AuthorityGeneration(u64);

impl AuthorityGeneration {
    const BEFORE_FIRST_OPEN: Self = Self(0);

    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }

    fn checked_next(self) -> Option<Self> {
        self.0.checked_add(1).map(Self)
    }
}

impl fmt::Display for AuthorityGeneration {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

/// The exact ephemeral authority/revision scope under which asynchronous work was requested.
///
/// Only a `DocumentSession` or one of its scoped observations can mint this value. It is useful
/// for render/search/diagnostic request provenance, but it is intentionally not a durable document
/// or history identity.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SessionAuthorityStamp {
    authority_generation: AuthorityGeneration,
    revision: DocumentRevision,
}

impl SessionAuthorityStamp {
    const fn new(authority_generation: AuthorityGeneration, revision: DocumentRevision) -> Self {
        Self {
            authority_generation,
            revision,
        }
    }

    #[must_use]
    pub const fn authority_generation(self) -> AuthorityGeneration {
        self.authority_generation
    }

    #[must_use]
    pub const fn revision(self) -> DocumentRevision {
        self.revision
    }
}

/// Semantic data scoped to one exact session authority generation and document revision.
///
/// The underlying engine observation remains engine-neutral and revision-stamped. The session
/// adds the product-owned authority generation after validating the engine/session revision
/// invariant. There is intentionally no public constructor: callers may transform an observation
/// while preserving provenance, but only `DocumentSession` can mint current authority scope.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SessionObservation<T> {
    authority_generation: AuthorityGeneration,
    engine_observation: SemanticObservation<T>,
}

impl<T> SessionObservation<T> {
    fn new(
        authority_generation: AuthorityGeneration,
        engine_observation: SemanticObservation<T>,
    ) -> Self {
        Self {
            authority_generation,
            engine_observation,
        }
    }

    #[must_use]
    pub const fn authority_generation(&self) -> AuthorityGeneration {
        self.authority_generation
    }

    #[must_use]
    pub const fn revision(&self) -> DocumentRevision {
        self.engine_observation.revision()
    }

    #[must_use]
    pub const fn authority_stamp(&self) -> SessionAuthorityStamp {
        SessionAuthorityStamp::new(self.authority_generation, self.revision())
    }

    #[must_use]
    pub const fn value(&self) -> &T {
        self.engine_observation.value()
    }

    #[must_use]
    pub fn into_value(self) -> T {
        self.engine_observation.into_value()
    }

    #[must_use]
    pub fn map<U>(self, map: impl FnOnce(T) -> U) -> SessionObservation<U> {
        SessionObservation::new(self.authority_generation, self.engine_observation.map(map))
    }
}

/// Why previously scoped work cannot be consumed as current application state.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AuthorityFreshnessError {
    NoOpenDocument,
    AuthorityChanged {
        observed: AuthorityGeneration,
        current: AuthorityGeneration,
    },
    Stale {
        observed: DocumentRevision,
        current: DocumentRevision,
    },
}

impl fmt::Display for AuthorityFreshnessError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoOpenDocument => formatter.write_str("no authoritative document is open"),
            Self::AuthorityChanged { observed, current } => write!(
                formatter,
                "work belongs to authority generation {observed}, current generation is {current}"
            ),
            Self::Stale { observed, current } => write!(
                formatter,
                "work is stale: observed revision {observed}, current revision {current}"
            ),
        }
    }
}

impl std::error::Error for AuthorityFreshnessError {}

/// Backwards-compatible name for semantic-observation freshness errors.
pub type ObservationFreshnessError = AuthorityFreshnessError;

/// Session-local ordering for captured recovery checkpoints.
///
/// This is not a persistent document identity and is not globally unique. Its only contract is
/// monotonic ordering inside one retained `DocumentSession` instance.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct CheckpointSequence(u64);

impl CheckpointSequence {
    const BEFORE_FIRST_CHECKPOINT: Self = Self(0);

    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }

    fn checked_next(self) -> Option<Self> {
        self.0.checked_add(1).map(Self)
    }
}

impl fmt::Display for CheckpointSequence {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

/// Product-owned checkpoint lineage around a payload captured from current authority.
///
/// The generic payload deliberately keeps this type independent of Writer, UNO and file-format
/// identifiers. R0A uses `String` only to qualify orchestration with the existing text fixture;
/// production recovery can later bind the same lineage rules to file/checkpoint artifacts plus a
/// recoverable operation journal.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RecoveryCheckpoint<T> {
    sequence: CheckpointSequence,
    source: SessionAuthorityStamp,
    value: T,
}

impl<T> RecoveryCheckpoint<T> {
    fn new(sequence: CheckpointSequence, source: SessionAuthorityStamp, value: T) -> Self {
        Self {
            sequence,
            source,
            value,
        }
    }

    #[must_use]
    pub const fn sequence(&self) -> CheckpointSequence {
        self.sequence
    }

    #[must_use]
    pub const fn source(&self) -> SessionAuthorityStamp {
        self.source
    }

    #[must_use]
    pub const fn value(&self) -> &T {
        &self.value
    }

    #[must_use]
    pub fn into_value(self) -> T {
        self.value
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CheckpointCaptureError {
    NotCurrent(AuthorityFreshnessError),
    SequenceExhausted,
}

impl fmt::Display for CheckpointCaptureError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotCurrent(error) => {
                write!(formatter, "cannot checkpoint stale authority: {error}")
            }
            Self::SequenceExhausted => {
                formatter.write_str("recovery checkpoint sequence exhausted")
            }
        }
    }
}

impl std::error::Error for CheckpointCaptureError {}

/// Evidence that a checkpoint was successfully rebound to a replacement authority.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RecoveryApplied {
    checkpoint_sequence: CheckpointSequence,
    source: SessionAuthorityStamp,
    recovered: SessionAuthorityStamp,
}

impl RecoveryApplied {
    fn new(
        checkpoint_sequence: CheckpointSequence,
        source: SessionAuthorityStamp,
        recovered: SessionAuthorityStamp,
    ) -> Self {
        Self {
            checkpoint_sequence,
            source,
            recovered,
        }
    }

    #[must_use]
    pub const fn checkpoint_sequence(self) -> CheckpointSequence {
        self.checkpoint_sequence
    }

    #[must_use]
    pub const fn source(self) -> SessionAuthorityStamp {
        self.source
    }

    #[must_use]
    pub const fn recovered(self) -> SessionAuthorityStamp {
        self.recovered
    }
}

pub struct DocumentSession<E> {
    engine: E,
    authority_generation: AuthorityGeneration,
    revision: Option<DocumentRevision>,
    last_checkpoint_sequence: CheckpointSequence,
}

impl<E: DocumentEngine> DocumentSession<E> {
    #[must_use]
    pub const fn new(engine: E) -> Self {
        Self {
            engine,
            authority_generation: AuthorityGeneration::BEFORE_FIRST_OPEN,
            revision: None,
            last_checkpoint_sequence: CheckpointSequence::BEFORE_FIRST_CHECKPOINT,
        }
    }

    pub fn open_text_fixture(&mut self, text: String) -> Result<DocumentRevision, EngineError> {
        // Reserve the next generation before asking the engine to replace authority. This makes
        // exhaustion fail without touching the current engine binding, while a normal engine
        // failure leaves the current generation/revision unchanged.
        let next_authority = self.authority_generation.checked_next().ok_or_else(|| {
            EngineError::Internal(String::from("document authority generation exhausted"))
        })?;
        let revision = self.engine.open_text_fixture(text)?;
        self.authority_generation = next_authority;
        self.revision = Some(revision);
        Ok(revision)
    }

    /// Replaces the engine binding after worker/engine loss and immediately withdraws authority.
    ///
    /// The old generation is not incremented merely because a process object was replaced. A new
    /// generation is committed only after the replacement engine successfully opens/restores a
    /// document. Until then all previously stamped work is rejected as having no open authority.
    pub fn replace_engine_after_authority_loss(&mut self, engine: E) {
        self.engine = engine;
        self.revision = None;
    }

    #[must_use]
    pub const fn known_revision(&self) -> Option<DocumentRevision> {
        self.revision
    }

    #[must_use]
    pub const fn known_authority_generation(&self) -> Option<AuthorityGeneration> {
        if self.revision.is_some() {
            Some(self.authority_generation)
        } else {
            None
        }
    }

    #[must_use]
    pub const fn current_authority_stamp(&self) -> Option<SessionAuthorityStamp> {
        match self.revision {
            Some(revision) => Some(SessionAuthorityStamp::new(
                self.authority_generation,
                revision,
            )),
            None => None,
        }
    }

    /// Reads semantic text and scopes it to this exact session authority incarnation.
    ///
    /// A successful read is coherent at the instant it is returned. Callers that retain the
    /// observation across work or await points must call [`Self::require_current`] before using
    /// it to affect current UI/application state.
    pub fn semantic_text(&self) -> Result<SessionObservation<String>, EngineError> {
        let observation = self.engine.semantic_text()?;
        let Some(current) = self.revision else {
            return Err(EngineError::Internal(String::from(
                "engine returned semantic state while session had no authoritative revision",
            )));
        };
        if observation.revision() != current {
            return Err(EngineError::Internal(format!(
                "engine semantic revision {} disagrees with session revision {current}",
                observation.revision()
            )));
        }
        Ok(SessionObservation::new(
            self.authority_generation,
            observation,
        ))
    }

    /// Rejects any authority stamp that no longer matches the exact live authority/revision.
    ///
    /// This is the allocation-free gate for future asynchronous render, search, diagnostics and
    /// other work that does not itself carry a semantic `SessionObservation`.
    pub fn require_current_stamp(
        &self,
        stamp: SessionAuthorityStamp,
    ) -> Result<(), AuthorityFreshnessError> {
        let current_revision = self
            .revision
            .ok_or(AuthorityFreshnessError::NoOpenDocument)?;
        if stamp.authority_generation() != self.authority_generation {
            return Err(AuthorityFreshnessError::AuthorityChanged {
                observed: stamp.authority_generation(),
                current: self.authority_generation,
            });
        }
        if stamp.revision() != current_revision {
            return Err(AuthorityFreshnessError::Stale {
                observed: stamp.revision(),
                current: current_revision,
            });
        }
        Ok(())
    }

    /// Rejects an observation from another authority generation or an older revision.
    pub fn require_current<T>(
        &self,
        observation: &SessionObservation<T>,
    ) -> Result<(), ObservationFreshnessError> {
        self.require_current_stamp(observation.authority_stamp())
    }

    /// Captures a payload and explicit checkpoint lineage from a current session observation.
    ///
    /// A stale observation cannot mint a checkpoint. Sequence allocation happens only after the
    /// freshness check succeeds, so rejected captures do not create gaps.
    pub fn capture_recovery_checkpoint<T: Clone>(
        &mut self,
        observation: &SessionObservation<T>,
    ) -> Result<RecoveryCheckpoint<T>, CheckpointCaptureError> {
        self.require_current(observation)
            .map_err(CheckpointCaptureError::NotCurrent)?;
        let sequence = self
            .last_checkpoint_sequence
            .checked_next()
            .ok_or(CheckpointCaptureError::SequenceExhausted)?;
        self.last_checkpoint_sequence = sequence;
        Ok(RecoveryCheckpoint::new(
            sequence,
            observation.authority_stamp(),
            observation.value().clone(),
        ))
    }

    /// R0A qualification adapter that restores a captured text payload into replacement authority.
    ///
    /// This deliberately reuses `open_text_fixture` rather than introducing a fake production
    /// checkpoint format into `DocumentEngine`. Production recovery will later open a persisted
    /// checkpoint artifact and replay accepted operations behind the same session-level lineage
    /// and authority rules.
    pub fn recover_text_fixture_from_checkpoint(
        &mut self,
        checkpoint: &RecoveryCheckpoint<String>,
    ) -> Result<RecoveryApplied, EngineError> {
        self.open_text_fixture(checkpoint.value().clone())?;
        let recovered = self.current_authority_stamp().ok_or_else(|| {
            EngineError::Internal(String::from(
                "successful checkpoint recovery did not publish session authority",
            ))
        })?;
        Ok(RecoveryApplied::new(
            checkpoint.sequence(),
            checkpoint.source(),
            recovered,
        ))
    }

    pub fn apply_transaction(
        &mut self,
        transaction: DocumentTransaction,
    ) -> Result<TransactionApplied, EngineError> {
        let applied = self.engine.apply_transaction(transaction)?;
        self.revision = Some(applied.new_revision);
        Ok(applied)
    }
}

#[cfg(test)]
mod tests {
    use document_protocol::{
        DocumentCapability, EngineCapabilities, ProtocolError, ProtocolVersion, TextEdit,
        TextOffset, TransactionLimits,
    };

    use super::*;

    const TEST_LIMITS: TransactionLimits = TransactionLimits::new(16, 1024, 4096);

    #[derive(Default)]
    struct TestEngine {
        text: Option<String>,
        revision: DocumentRevision,
        fail_next_open: bool,
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
            if self.fail_next_open {
                self.fail_next_open = false;
                return Err(EngineError::Internal(String::from("injected open failure")));
            }
            self.text = Some(text);
            self.revision = DocumentRevision::INITIAL;
            Ok(self.revision)
        }

        fn semantic_text(&self) -> Result<SemanticObservation<String>, EngineError> {
            let value = self.text.clone().ok_or(EngineError::NotOpen)?;
            Ok(SemanticObservation::new(self.revision, value))
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
            let mut edits = transaction.edits;
            edits.sort_by_key(|edit| edit.start_utf8);
            for edit in edits.into_iter().rev() {
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

    fn replace_first_character(expected_revision: DocumentRevision) -> DocumentTransaction {
        DocumentTransaction {
            expected_revision,
            edits: vec![TextEdit {
                start_utf8: TextOffset::new(0),
                end_utf8: TextOffset::new(1),
                replacement: String::from("A"),
            }],
        }
    }

    #[test]
    fn semantic_read_is_stamped_with_session_authority_and_revision() {
        let mut session = DocumentSession::new(TestEngine::default());
        assert_eq!(session.known_authority_generation(), None);
        session.open_text_fixture(String::from("abc")).unwrap();
        let generation = session.known_authority_generation().unwrap();

        let observation = session.semantic_text().unwrap();
        let stamp = observation.authority_stamp();

        assert_eq!(observation.authority_generation(), generation);
        assert_eq!(observation.revision(), DocumentRevision::INITIAL);
        assert_eq!(observation.value().as_str(), "abc");
        assert_eq!(session.current_authority_stamp(), Some(stamp));
        assert_eq!(session.require_current_stamp(stamp), Ok(()));
        assert_eq!(session.require_current(&observation), Ok(()));

        let length = observation.map(|text| text.len());
        assert_eq!(length.authority_generation(), generation);
        assert_eq!(length.revision(), DocumentRevision::INITIAL);
        assert_eq!(*length.value(), 3);
        assert_eq!(session.require_current(&length), Ok(()));
    }

    #[test]
    fn retained_observation_and_stamp_are_rejected_after_authoritative_mutation() {
        let mut session = DocumentSession::new(TestEngine::default());
        session.open_text_fixture(String::from("abc")).unwrap();
        let generation = session.known_authority_generation().unwrap();
        let old = session.semantic_text().unwrap();
        let old_stamp = old.authority_stamp();

        session
            .apply_transaction(replace_first_character(DocumentRevision::INITIAL))
            .unwrap();

        assert_eq!(session.known_authority_generation(), Some(generation));
        let expected = AuthorityFreshnessError::Stale {
            observed: DocumentRevision::INITIAL,
            current: DocumentRevision::new(1),
        };
        assert_eq!(session.require_current(&old), Err(expected));
        assert_eq!(session.require_current_stamp(old_stamp), Err(expected));

        let current = session.semantic_text().unwrap();
        assert_eq!(current.authority_generation(), generation);
        assert_eq!(current.revision(), DocumentRevision::new(1));
        assert_eq!(current.value().as_str(), "Abc");
        assert_eq!(session.require_current(&current), Ok(()));
    }

    #[test]
    fn old_r0_observation_is_rejected_after_successful_reopen_at_r0() {
        let mut session = DocumentSession::new(TestEngine::default());
        session.open_text_fixture(String::from("first")).unwrap();
        let first_generation = session.known_authority_generation().unwrap();
        let old = session.semantic_text().unwrap();
        assert_eq!(old.revision(), DocumentRevision::INITIAL);

        let reopened_revision = session.open_text_fixture(String::from("second")).unwrap();
        let second_generation = session.known_authority_generation().unwrap();

        assert_eq!(reopened_revision, DocumentRevision::INITIAL);
        assert_ne!(first_generation, second_generation);
        assert_eq!(
            session.require_current(&old),
            Err(AuthorityFreshnessError::AuthorityChanged {
                observed: first_generation,
                current: second_generation,
            })
        );

        let current = session.semantic_text().unwrap();
        assert_eq!(current.authority_generation(), second_generation);
        assert_eq!(current.revision(), DocumentRevision::INITIAL);
        assert_eq!(current.value().as_str(), "second");
        assert_eq!(session.require_current(&current), Ok(()));
    }

    #[test]
    fn recovery_checkpoint_requires_current_observation_and_sequences_successes_only() {
        let mut session = DocumentSession::new(TestEngine::default());
        session.open_text_fixture(String::from("abc")).unwrap();
        let old = session.semantic_text().unwrap();
        session
            .apply_transaction(replace_first_character(DocumentRevision::INITIAL))
            .unwrap();

        assert_eq!(
            session.capture_recovery_checkpoint(&old),
            Err(CheckpointCaptureError::NotCurrent(
                AuthorityFreshnessError::Stale {
                    observed: DocumentRevision::INITIAL,
                    current: DocumentRevision::new(1),
                }
            ))
        );

        let current = session.semantic_text().unwrap();
        let first = session.capture_recovery_checkpoint(&current).unwrap();
        let second = session.capture_recovery_checkpoint(&current).unwrap();

        assert_eq!(first.sequence().get(), 1);
        assert_eq!(second.sequence().get(), 2);
        assert_eq!(first.source(), current.authority_stamp());
        assert_eq!(first.value().as_str(), "Abc");
    }

    #[test]
    fn checkpoint_recovery_replaces_lost_authority_and_rejects_old_async_work() {
        let mut session = DocumentSession::new(TestEngine::default());
        session.open_text_fixture(String::from("abc")).unwrap();
        session
            .apply_transaction(replace_first_character(DocumentRevision::INITIAL))
            .unwrap();
        let source = session.semantic_text().unwrap();
        let source_stamp = source.authority_stamp();
        assert_eq!(source_stamp.revision(), DocumentRevision::new(1));
        let checkpoint = session.capture_recovery_checkpoint(&source).unwrap();

        session.replace_engine_after_authority_loss(TestEngine::default());

        assert_eq!(session.known_revision(), None);
        assert_eq!(session.known_authority_generation(), None);
        assert_eq!(
            session.require_current_stamp(source_stamp),
            Err(AuthorityFreshnessError::NoOpenDocument)
        );
        assert_eq!(
            session.require_current(&source),
            Err(AuthorityFreshnessError::NoOpenDocument)
        );

        let applied = session
            .recover_text_fixture_from_checkpoint(&checkpoint)
            .unwrap();
        let recovered_stamp = session.current_authority_stamp().unwrap();

        assert_eq!(applied.checkpoint_sequence(), checkpoint.sequence());
        assert_eq!(applied.source(), source_stamp);
        assert_eq!(applied.recovered(), recovered_stamp);
        assert_ne!(
            recovered_stamp.authority_generation(),
            source_stamp.authority_generation()
        );
        assert_eq!(recovered_stamp.revision(), DocumentRevision::INITIAL);
        assert_eq!(session.semantic_text().unwrap().value().as_str(), "Abc");
        assert_eq!(
            session.require_current_stamp(source_stamp),
            Err(AuthorityFreshnessError::AuthorityChanged {
                observed: source_stamp.authority_generation(),
                current: recovered_stamp.authority_generation(),
            })
        );
    }

    #[test]
    fn failed_recovery_does_not_publish_replacement_authority() {
        let mut session = DocumentSession::new(TestEngine::default());
        session.open_text_fixture(String::from("abc")).unwrap();
        let source = session.semantic_text().unwrap();
        let source_generation = source.authority_generation();
        let checkpoint = session.capture_recovery_checkpoint(&source).unwrap();

        session.replace_engine_after_authority_loss(TestEngine {
            fail_next_open: true,
            ..TestEngine::default()
        });

        let error = session
            .recover_text_fixture_from_checkpoint(&checkpoint)
            .unwrap_err();

        assert!(matches!(error, EngineError::Internal(message) if message.contains("injected")));
        assert_eq!(session.known_revision(), None);
        assert_eq!(session.known_authority_generation(), None);
        assert_eq!(session.current_authority_stamp(), None);
        assert_eq!(
            session.require_current(&source),
            Err(AuthorityFreshnessError::NoOpenDocument)
        );

        let applied = session
            .recover_text_fixture_from_checkpoint(&checkpoint)
            .unwrap();
        assert_eq!(
            applied.recovered().authority_generation().get(),
            source_generation.get() + 1
        );
    }

    #[test]
    fn rejected_transaction_does_not_invalidate_current_observation() {
        let mut session = DocumentSession::new(TestEngine::default());
        session.open_text_fixture(String::from("abc")).unwrap();
        let generation = session.known_authority_generation().unwrap();
        let observation = session.semantic_text().unwrap();

        let error = session
            .apply_transaction(DocumentTransaction {
                expected_revision: DocumentRevision::new(99),
                edits: Vec::new(),
            })
            .unwrap_err();

        assert!(matches!(
            error,
            EngineError::Protocol(ProtocolError::RevisionConflict { .. })
        ));
        assert_eq!(session.known_revision(), Some(DocumentRevision::INITIAL));
        assert_eq!(session.known_authority_generation(), Some(generation));
        assert_eq!(session.require_current(&observation), Ok(()));
    }

    #[test]
    fn failed_open_does_not_replace_current_authority() {
        let mut session = DocumentSession::new(TestEngine::default());
        session.open_text_fixture(String::from("abc")).unwrap();
        let generation = session.known_authority_generation().unwrap();
        let observation = session.semantic_text().unwrap();
        session.engine.fail_next_open = true;

        let error = session
            .open_text_fixture(String::from("replacement"))
            .unwrap_err();

        assert!(matches!(error, EngineError::Internal(message) if message.contains("injected")));
        assert_eq!(session.known_revision(), Some(DocumentRevision::INITIAL));
        assert_eq!(session.known_authority_generation(), Some(generation));
        assert_eq!(session.require_current(&observation), Ok(()));
        assert_eq!(session.semantic_text().unwrap().value().as_str(), "abc");
    }

    #[test]
    fn semantic_read_rejects_engine_session_revision_disagreement() {
        let mut session = DocumentSession::new(TestEngine::default());
        session.open_text_fixture(String::from("abc")).unwrap();
        session.engine.revision = DocumentRevision::new(5);

        let error = session.semantic_text().unwrap_err();

        assert!(matches!(error, EngineError::Internal(message) if message.contains("disagrees")));
    }
}
