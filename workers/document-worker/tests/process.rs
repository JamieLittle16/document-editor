use std::io::Write;
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

use document_protocol::{ProtocolVersion, RequestId};
use document_transport::{Frame, FrameKind, read_frame, write_frame};
use document_worker::{
    R0A_CAPABILITIES_COMMAND, R0A_SHUTDOWN_COMMAND, R0A_SPIKE_FRAME_LIMITS, R0A_STATUS_OK,
    R0A_STDIO_SPIKE_ARG,
};

struct WorkerProcess {
    child: Child,
    input: Option<ChildStdin>,
    output: ChildStdout,
}

impl WorkerProcess {
    fn spawn() -> Self {
        let mut child = Command::new(env!("CARGO_BIN_EXE_document-worker"))
            .arg(R0A_STDIO_SPIKE_ARG)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .expect("spawn document-worker spike");
        let input = child.stdin.take().expect("worker stdin");
        let output = child.stdout.take().expect("worker stdout");
        Self {
            child,
            input: Some(input),
            output,
        }
    }

    fn request(&mut self, request_id: u64, payload: Vec<u8>) -> Frame {
        let input = self.input.as_mut().expect("worker input still open");
        write_frame(
            input,
            &Frame::new(FrameKind::Request, RequestId::new(request_id), payload),
            R0A_SPIKE_FRAME_LIMITS,
        )
        .expect("write worker request");
        input.flush().expect("flush worker request");
        read_frame(&mut self.output, R0A_SPIKE_FRAME_LIMITS)
            .expect("read worker response")
            .expect("worker response before EOF")
    }

    fn graceful_shutdown(mut self, request_id: u64) {
        let response = self.request(request_id, vec![R0A_SHUTDOWN_COMMAND]);
        assert_eq!(response.kind, FrameKind::Response);
        assert_eq!(response.request_id, RequestId::new(request_id));
        assert_eq!(response.payload, vec![R0A_STATUS_OK, R0A_SHUTDOWN_COMMAND]);
        drop(self.input.take());
        let status = self.child.wait().expect("wait for graceful worker exit");
        assert!(status.success(), "worker exited unsuccessfully: {status}");
        assert!(
            read_frame(&mut self.output, R0A_SPIKE_FRAME_LIMITS)
                .expect("clean EOF after worker shutdown")
                .is_none()
        );
    }
}

fn assert_capabilities_response(response: &Frame, request_id: u64) {
    assert_eq!(response.kind, FrameKind::Response);
    assert_eq!(response.request_id, RequestId::new(request_id));
    assert_eq!(response.payload.len(), 6);
    assert_eq!(response.payload[0], R0A_STATUS_OK);
    assert_eq!(response.payload[1], R0A_CAPABILITIES_COMMAND);
    let major = u16::from_le_bytes([response.payload[2], response.payload[3]]);
    let minor = u16::from_le_bytes([response.payload[4], response.payload[5]]);
    assert_eq!(ProtocolVersion { major, minor }, ProtocolVersion::V0);
}

#[test]
fn real_child_process_correlates_request_and_shuts_down_cleanly() {
    let mut worker = WorkerProcess::spawn();
    let response = worker.request(0x1122_3344_5566_7788, vec![R0A_CAPABILITIES_COMMAND]);
    assert_capabilities_response(&response, 0x1122_3344_5566_7788);
    worker.graceful_shutdown(2);
}

#[test]
fn stdin_eof_is_a_clean_worker_exit() {
    let mut worker = WorkerProcess::spawn();
    drop(worker.input.take());
    let status = worker
        .child
        .wait()
        .expect("wait for EOF-driven worker exit");
    assert!(status.success(), "worker exited unsuccessfully: {status}");
    assert!(
        read_frame(&mut worker.output, R0A_SPIKE_FRAME_LIMITS)
            .expect("read clean worker EOF")
            .is_none()
    );
}

#[test]
fn forced_worker_death_is_observable_and_fresh_process_can_restart() {
    let mut first = WorkerProcess::spawn();
    let response = first.request(10, vec![R0A_CAPABILITIES_COMMAND]);
    assert_capabilities_response(&response, 10);

    first.child.kill().expect("force-kill worker");
    let status = first.child.wait().expect("wait for killed worker");
    assert!(
        !status.success(),
        "force-killed worker unexpectedly succeeded"
    );
    drop(first.input.take());
    assert!(
        read_frame(&mut first.output, R0A_SPIKE_FRAME_LIMITS)
            .expect("EOF after killed worker")
            .is_none()
    );

    let mut restarted = WorkerProcess::spawn();
    let response = restarted.request(11, vec![R0A_CAPABILITIES_COMMAND]);
    assert_capabilities_response(&response, 11);
    restarted.graceful_shutdown(12);
}
