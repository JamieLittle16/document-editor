use std::env;
use std::io;
use std::process::ExitCode;

use document_engine_api::DocumentEngine;
use document_engine_mock::MockDocumentEngine;
use document_worker::{run_r0a_stdio_spike, R0A_STDIO_SPIKE_ARG};

fn main() -> ExitCode {
    let mut arguments = env::args().skip(1);
    match arguments.next() {
        Some(argument) if argument == R0A_STDIO_SPIKE_ARG && arguments.next().is_none() => {
            let stdin = io::stdin();
            let stdout = io::stdout();
            let mut input = stdin.lock();
            let mut output = stdout.lock();
            match run_r0a_stdio_spike(&mut input, &mut output) {
                Ok(()) => ExitCode::SUCCESS,
                Err(error) => {
                    eprintln!("document-worker R0A stdio spike failed: {error}");
                    ExitCode::from(1)
                }
            }
        }
        None => {
            let engine = MockDocumentEngine::default();
            let capabilities = engine.capabilities();
            println!(
                "document-worker protocol {}.{}",
                capabilities.protocol.major, capabilities.protocol.minor
            );
            ExitCode::SUCCESS
        }
        Some(_) => {
            eprintln!("usage: document-worker [{R0A_STDIO_SPIKE_ARG}]");
            ExitCode::from(2)
        }
    }
}
