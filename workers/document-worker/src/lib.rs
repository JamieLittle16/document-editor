#![doc = "R0A document-worker process spike. The stdio command codec is deliberately provisional."]

use std::fmt;
use std::io::{self, Read, Write};

use document_engine_api::DocumentEngine;
use document_engine_mock::MockDocumentEngine;
use document_transport::{read_frame, write_frame, Frame, FrameKind, FrameLimits, TransportError};

/// Enables the disposable R0A stdin/stdout process protocol.
pub const R0A_STDIO_SPIKE_ARG: &str = "--r0a-stdio-spike";
/// Provisional command byte requesting the mock engine's protocol capabilities.
pub const R0A_CAPABILITIES_COMMAND: u8 = 1;
/// Provisional command byte requesting a graceful worker shutdown.
pub const R0A_SHUTDOWN_COMMAND: u8 = 2;
/// Success status byte used only by the R0A command codec.
pub const R0A_STATUS_OK: u8 = 0;
/// Invalid-command status byte used only by the R0A command codec.
pub const R0A_STATUS_INVALID_REQUEST: u8 = 1;
/// Explicitly small control-plane bound for the disposable worker spike.
pub const R0A_SPIKE_FRAME_LIMITS: FrameLimits = FrameLimits::new(1024);

#[derive(Debug)]
pub enum WorkerSpikeError {
    Transport(TransportError),
    Io(io::Error),
    UnexpectedFrameKind(FrameKind),
}

impl fmt::Display for WorkerSpikeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Transport(error) => write!(formatter, "worker transport error: {error}"),
            Self::Io(error) => write!(formatter, "worker I/O error: {error}"),
            Self::UnexpectedFrameKind(kind) => {
                write!(formatter, "worker received unexpected {kind:?} frame")
            }
        }
    }
}

impl std::error::Error for WorkerSpikeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Transport(error) => Some(error),
            Self::Io(error) => Some(error),
            Self::UnexpectedFrameKind(_) => None,
        }
    }
}

impl From<TransportError> for WorkerSpikeError {
    fn from(value: TransportError) -> Self {
        Self::Transport(value)
    }
}

impl From<io::Error> for WorkerSpikeError {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}

/// Runs the disposable R0A worker loop over an arbitrary byte stream.
///
/// This proves framing/process semantics only. The command payload format is not the engine
/// protocol v1 wire format and must not be consumed by product features or persisted data.
pub fn run_r0a_stdio_spike<R: Read, W: Write>(
    input: &mut R,
    output: &mut W,
) -> Result<(), WorkerSpikeError> {
    let engine = MockDocumentEngine::default();

    loop {
        let Some(frame) = read_frame(input, R0A_SPIKE_FRAME_LIMITS)? else {
            return Ok(());
        };

        if frame.kind != FrameKind::Request {
            return Err(WorkerSpikeError::UnexpectedFrameKind(frame.kind));
        }

        let (payload, should_shutdown) = handle_spike_request(&engine, &frame.payload);
        let response = Frame::new(FrameKind::Response, frame.request_id, payload);
        write_frame(output, &response, R0A_SPIKE_FRAME_LIMITS)?;
        output.flush()?;

        if should_shutdown {
            return Ok(());
        }
    }
}

fn handle_spike_request(engine: &MockDocumentEngine, payload: &[u8]) -> (Vec<u8>, bool) {
    match payload {
        [R0A_CAPABILITIES_COMMAND] => {
            let capabilities = engine.capabilities();
            let mut response = Vec::with_capacity(6);
            response.push(R0A_STATUS_OK);
            response.push(R0A_CAPABILITIES_COMMAND);
            response.extend_from_slice(&capabilities.protocol.major.to_le_bytes());
            response.extend_from_slice(&capabilities.protocol.minor.to_le_bytes());
            (response, false)
        }
        [R0A_SHUTDOWN_COMMAND] => (
            vec![R0A_STATUS_OK, R0A_SHUTDOWN_COMMAND],
            true,
        ),
        [command] => (
            vec![R0A_STATUS_INVALID_REQUEST, *command],
            false,
        ),
        _ => (vec![R0A_STATUS_INVALID_REQUEST, 0], false),
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use document_protocol::RequestId;

    use super::*;

    fn request_bytes(id: u64, payload: Vec<u8>) -> Vec<u8> {
        let mut bytes = Vec::new();
        write_frame(
            &mut bytes,
            &Frame::new(FrameKind::Request, RequestId::new(id), payload),
            R0A_SPIKE_FRAME_LIMITS,
        )
        .unwrap();
        bytes
    }

    #[test]
    fn in_memory_capabilities_response_preserves_request_id() {
        let mut input = Cursor::new(request_bytes(41, vec![R0A_CAPABILITIES_COMMAND]));
        let mut output = Vec::new();

        run_r0a_stdio_spike(&mut input, &mut output).unwrap();

        let response = read_frame(&mut Cursor::new(output), R0A_SPIKE_FRAME_LIMITS)
            .unwrap()
            .unwrap();
        assert_eq!(response.kind, FrameKind::Response);
        assert_eq!(response.request_id, RequestId::new(41));
        assert_eq!(response.payload[0], R0A_STATUS_OK);
        assert_eq!(response.payload[1], R0A_CAPABILITIES_COMMAND);
    }

    #[test]
    fn shutdown_stops_before_processing_later_frames() {
        let mut input_bytes = request_bytes(1, vec![R0A_SHUTDOWN_COMMAND]);
        input_bytes.extend(request_bytes(2, vec![R0A_CAPABILITIES_COMMAND]));
        let mut input = Cursor::new(input_bytes);
        let mut output = Vec::new();

        run_r0a_stdio_spike(&mut input, &mut output).unwrap();

        let mut responses = Cursor::new(output);
        let response = read_frame(&mut responses, R0A_SPIKE_FRAME_LIMITS)
            .unwrap()
            .unwrap();
        assert_eq!(response.request_id, RequestId::new(1));
        assert_eq!(response.payload, vec![R0A_STATUS_OK, R0A_SHUTDOWN_COMMAND]);
        assert!(
            read_frame(&mut responses, R0A_SPIKE_FRAME_LIMITS)
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn response_frame_from_host_is_rejected() {
        let mut bytes = Vec::new();
        write_frame(
            &mut bytes,
            &Frame::new(FrameKind::Response, RequestId::new(7), Vec::new()),
            R0A_SPIKE_FRAME_LIMITS,
        )
        .unwrap();

        let error = run_r0a_stdio_spike(&mut Cursor::new(bytes), &mut Vec::new()).unwrap_err();
        assert!(matches!(
            error,
            WorkerSpikeError::UnexpectedFrameKind(FrameKind::Response)
        ));
    }

    #[test]
    fn invalid_command_is_explicit_response_not_worker_crash() {
        let mut input = Cursor::new(request_bytes(9, vec![99]));
        let mut output = Vec::new();

        run_r0a_stdio_spike(&mut input, &mut output).unwrap();

        let response = read_frame(&mut Cursor::new(output), R0A_SPIKE_FRAME_LIMITS)
            .unwrap()
            .unwrap();
        assert_eq!(response.request_id, RequestId::new(9));
        assert_eq!(response.payload, vec![R0A_STATUS_INVALID_REQUEST, 99]);
    }
}
