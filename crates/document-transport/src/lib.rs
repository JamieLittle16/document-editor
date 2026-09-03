#![doc = "Bounded stream framing for document-engine control-plane messages."]

use std::fmt;
use std::io::{self, Read, Write};

use document_protocol::RequestId;

const FRAME_MAGIC: [u8; 4] = *b"DETR";
pub const CONTROL_FRAME_VERSION: u16 = 1;
pub const CONTROL_FRAME_HEADER_BYTES: usize = 20;
const SUPPORTED_FLAGS: u8 = 0;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FrameKind {
    Request,
    Response,
}

impl FrameKind {
    const fn wire(self) -> u8 {
        match self {
            Self::Request => 1,
            Self::Response => 2,
        }
    }

    fn from_wire(value: u8) -> Result<Self, TransportError> {
        match value {
            1 => Ok(Self::Request),
            2 => Ok(Self::Response),
            other => Err(TransportError::UnknownFrameKind(other)),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FrameLimits {
    max_control_payload_bytes: u32,
}

impl FrameLimits {
    #[must_use]
    pub const fn new(max_control_payload_bytes: u32) -> Self {
        Self {
            max_control_payload_bytes,
        }
    }

    #[must_use]
    pub const fn max_control_payload_bytes(self) -> u32 {
        self.max_control_payload_bytes
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Frame {
    pub kind: FrameKind,
    pub request_id: RequestId,
    pub payload: Vec<u8>,
}

impl Frame {
    #[must_use]
    pub const fn new(kind: FrameKind, request_id: RequestId, payload: Vec<u8>) -> Self {
        Self {
            kind,
            request_id,
            payload,
        }
    }
}

#[derive(Debug)]
pub enum TransportError {
    Io(io::Error),
    BadMagic([u8; 4]),
    UnsupportedFrameVersion { actual: u16, supported: u16 },
    UnknownFrameKind(u8),
    UnsupportedFlags(u8),
    PayloadTooLarge { actual: u64, max: u32 },
    TruncatedHeader { received: usize },
    TruncatedPayload { expected: u32, received: u32 },
}

impl fmt::Display for TransportError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "transport I/O error: {error}"),
            Self::BadMagic(actual) => {
                write!(formatter, "invalid control-frame magic: {:02x?}", actual)
            }
            Self::UnsupportedFrameVersion { actual, supported } => write!(
                formatter,
                "unsupported control-frame version {actual}; supported version is {supported}"
            ),
            Self::UnknownFrameKind(kind) => {
                write!(formatter, "unknown control-frame kind {kind}")
            }
            Self::UnsupportedFlags(flags) => {
                write!(formatter, "unsupported control-frame flags 0x{flags:02x}")
            }
            Self::PayloadTooLarge { actual, max } => write!(
                formatter,
                "control-frame payload contains {actual} bytes; limit is {max}"
            ),
            Self::TruncatedHeader { received } => write!(
                formatter,
                "truncated control-frame header: received {received} of {CONTROL_FRAME_HEADER_BYTES} bytes"
            ),
            Self::TruncatedPayload { expected, received } => write!(
                formatter,
                "truncated control-frame payload: received {received} of {expected} bytes"
            ),
        }
    }
}

impl std::error::Error for TransportError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            _ => None,
        }
    }
}

impl From<io::Error> for TransportError {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}

pub fn write_frame<W: Write>(
    writer: &mut W,
    frame: &Frame,
    limits: FrameLimits,
) -> Result<(), TransportError> {
    let actual_payload_len = u64::try_from(frame.payload.len()).unwrap_or(u64::MAX);
    if actual_payload_len > u64::from(limits.max_control_payload_bytes) {
        return Err(TransportError::PayloadTooLarge {
            actual: actual_payload_len,
            max: limits.max_control_payload_bytes,
        });
    }

    let payload_len =
        u32::try_from(frame.payload.len()).map_err(|_| TransportError::PayloadTooLarge {
            actual: actual_payload_len,
            max: limits.max_control_payload_bytes,
        })?;

    let header = encode_header(frame.kind, frame.request_id, payload_len);
    writer.write_all(&header)?;
    writer.write_all(&frame.payload)?;
    Ok(())
}

pub fn read_frame<R: Read>(
    reader: &mut R,
    limits: FrameLimits,
) -> Result<Option<Frame>, TransportError> {
    let Some(header) = read_header(reader)? else {
        return Ok(None);
    };

    let actual_magic = [header[0], header[1], header[2], header[3]];
    if actual_magic != FRAME_MAGIC {
        return Err(TransportError::BadMagic(actual_magic));
    }

    let version = u16::from_le_bytes([header[4], header[5]]);
    if version != CONTROL_FRAME_VERSION {
        return Err(TransportError::UnsupportedFrameVersion {
            actual: version,
            supported: CONTROL_FRAME_VERSION,
        });
    }

    let kind = FrameKind::from_wire(header[6])?;
    let flags = header[7];
    if flags != SUPPORTED_FLAGS {
        return Err(TransportError::UnsupportedFlags(flags));
    }

    let request_id = RequestId::new(u64::from_le_bytes([
        header[8], header[9], header[10], header[11], header[12], header[13], header[14],
        header[15],
    ]));
    let payload_len = u32::from_le_bytes([header[16], header[17], header[18], header[19]]);
    if payload_len > limits.max_control_payload_bytes {
        return Err(TransportError::PayloadTooLarge {
            actual: u64::from(payload_len),
            max: limits.max_control_payload_bytes,
        });
    }

    let mut payload = vec![0_u8; payload_len as usize];
    read_payload(reader, &mut payload, payload_len)?;

    Ok(Some(Frame::new(kind, request_id, payload)))
}

fn encode_header(kind: FrameKind, request_id: RequestId, payload_len: u32) -> [u8; 20] {
    let mut header = [0_u8; CONTROL_FRAME_HEADER_BYTES];
    header[0..4].copy_from_slice(&FRAME_MAGIC);
    header[4..6].copy_from_slice(&CONTROL_FRAME_VERSION.to_le_bytes());
    header[6] = kind.wire();
    header[7] = SUPPORTED_FLAGS;
    header[8..16].copy_from_slice(&request_id.get().to_le_bytes());
    header[16..20].copy_from_slice(&payload_len.to_le_bytes());
    header
}

fn read_header<R: Read>(reader: &mut R) -> Result<Option<[u8; 20]>, TransportError> {
    let mut header = [0_u8; CONTROL_FRAME_HEADER_BYTES];
    let mut received = 0_usize;

    while received < header.len() {
        match reader.read(&mut header[received..]) {
            Ok(0) if received == 0 => return Ok(None),
            Ok(0) => return Err(TransportError::TruncatedHeader { received }),
            Ok(count) => received += count,
            Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
            Err(error) => return Err(error.into()),
        }
    }

    Ok(Some(header))
}

fn read_payload<R: Read>(
    reader: &mut R,
    payload: &mut [u8],
    expected: u32,
) -> Result<(), TransportError> {
    let mut received = 0_usize;

    while received < payload.len() {
        match reader.read(&mut payload[received..]) {
            Ok(0) => {
                return Err(TransportError::TruncatedPayload {
                    expected,
                    received: u32::try_from(received).unwrap_or(u32::MAX),
                });
            }
            Ok(count) => received += count,
            Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
            Err(error) => return Err(error.into()),
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::*;

    const TEST_LIMITS: FrameLimits = FrameLimits::new(32);

    struct ChunkedReader {
        inner: Cursor<Vec<u8>>,
        max_chunk: usize,
    }

    impl ChunkedReader {
        fn new(bytes: Vec<u8>, max_chunk: usize) -> Self {
            Self {
                inner: Cursor::new(bytes),
                max_chunk,
            }
        }
    }

    impl Read for ChunkedReader {
        fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
            let chunk = buffer.len().min(self.max_chunk);
            self.inner.read(&mut buffer[..chunk])
        }
    }

    struct ChunkedWriter {
        bytes: Vec<u8>,
        max_chunk: usize,
        writes: usize,
    }

    impl ChunkedWriter {
        fn new(max_chunk: usize) -> Self {
            Self {
                bytes: Vec::new(),
                max_chunk,
                writes: 0,
            }
        }
    }

    impl Write for ChunkedWriter {
        fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
            if buffer.is_empty() {
                return Ok(0);
            }
            let chunk = buffer.len().min(self.max_chunk);
            self.bytes.extend_from_slice(&buffer[..chunk]);
            self.writes += 1;
            Ok(chunk)
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    fn raw_header(kind: u8, version: u16, flags: u8, request_id: u64, payload_len: u32) -> Vec<u8> {
        let mut header = [0_u8; CONTROL_FRAME_HEADER_BYTES];
        header[0..4].copy_from_slice(&FRAME_MAGIC);
        header[4..6].copy_from_slice(&version.to_le_bytes());
        header[6] = kind;
        header[7] = flags;
        header[8..16].copy_from_slice(&request_id.to_le_bytes());
        header[16..20].copy_from_slice(&payload_len.to_le_bytes());
        header.to_vec()
    }

    #[test]
    fn request_round_trip_survives_short_reads_and_writes() {
        let frame = Frame::new(
            FrameKind::Request,
            RequestId::new(42),
            b"control-payload".to_vec(),
        );
        let mut writer = ChunkedWriter::new(2);
        write_frame(&mut writer, &frame, TEST_LIMITS).unwrap();
        assert!(writer.writes > 2);

        let mut reader = ChunkedReader::new(writer.bytes, 3);
        let decoded = read_frame(&mut reader, TEST_LIMITS).unwrap().unwrap();
        assert_eq!(decoded, frame);
        assert!(read_frame(&mut reader, TEST_LIMITS).unwrap().is_none());
    }

    #[test]
    fn response_preserves_request_correlation() {
        let frame = Frame::new(FrameKind::Response, RequestId::new(9001), Vec::new());
        let mut bytes = Vec::new();
        write_frame(&mut bytes, &frame, TEST_LIMITS).unwrap();
        let decoded = read_frame(&mut Cursor::new(bytes), TEST_LIMITS)
            .unwrap()
            .unwrap();

        assert_eq!(decoded.kind, FrameKind::Response);
        assert_eq!(decoded.request_id, RequestId::new(9001));
        assert!(decoded.payload.is_empty());
    }

    #[test]
    fn clean_eof_before_header_is_not_a_protocol_error() {
        assert!(
            read_frame(&mut Cursor::new(Vec::<u8>::new()), TEST_LIMITS)
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn partial_header_is_typed_truncation() {
        let bytes = FRAME_MAGIC.to_vec();
        let error = read_frame(&mut Cursor::new(bytes), TEST_LIMITS).unwrap_err();
        assert!(matches!(
            error,
            TransportError::TruncatedHeader { received: 4 }
        ));
    }

    #[test]
    fn partial_payload_is_typed_truncation() {
        let mut bytes = raw_header(1, CONTROL_FRAME_VERSION, 0, 7, 3);
        bytes.extend_from_slice(b"ab");
        let error = read_frame(&mut Cursor::new(bytes), TEST_LIMITS).unwrap_err();
        assert!(matches!(
            error,
            TransportError::TruncatedPayload {
                expected: 3,
                received: 2
            }
        ));
    }

    #[test]
    fn oversized_announced_payload_is_rejected_before_payload_read() {
        let bytes = raw_header(1, CONTROL_FRAME_VERSION, 0, 7, 33);
        let error = read_frame(&mut Cursor::new(bytes), TEST_LIMITS).unwrap_err();
        assert!(matches!(
            error,
            TransportError::PayloadTooLarge {
                actual: 33,
                max: 32
            }
        ));
    }

    #[test]
    fn writer_rejects_oversized_payload() {
        let frame = Frame::new(FrameKind::Request, RequestId::new(1), vec![0_u8; 33]);
        let error = write_frame(&mut Vec::new(), &frame, TEST_LIMITS).unwrap_err();
        assert!(matches!(
            error,
            TransportError::PayloadTooLarge {
                actual: 33,
                max: 32
            }
        ));
    }

    #[test]
    fn bad_magic_is_rejected() {
        let mut bytes = raw_header(1, CONTROL_FRAME_VERSION, 0, 1, 0);
        bytes[0] = b'X';
        let error = read_frame(&mut Cursor::new(bytes), TEST_LIMITS).unwrap_err();
        assert!(matches!(error, TransportError::BadMagic(_)));
    }

    #[test]
    fn unsupported_version_is_rejected() {
        let bytes = raw_header(1, CONTROL_FRAME_VERSION + 1, 0, 1, 0);
        let error = read_frame(&mut Cursor::new(bytes), TEST_LIMITS).unwrap_err();
        assert!(matches!(
            error,
            TransportError::UnsupportedFrameVersion { .. }
        ));
    }

    #[test]
    fn unknown_kind_is_rejected() {
        let bytes = raw_header(99, CONTROL_FRAME_VERSION, 0, 1, 0);
        let error = read_frame(&mut Cursor::new(bytes), TEST_LIMITS).unwrap_err();
        assert!(matches!(error, TransportError::UnknownFrameKind(99)));
    }

    #[test]
    fn unsupported_flags_are_rejected() {
        let bytes = raw_header(1, CONTROL_FRAME_VERSION, 1, 1, 0);
        let error = read_frame(&mut Cursor::new(bytes), TEST_LIMITS).unwrap_err();
        assert!(matches!(error, TransportError::UnsupportedFlags(1)));
    }
}
