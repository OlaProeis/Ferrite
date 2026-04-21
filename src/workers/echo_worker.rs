//! Echo worker - proof-of-concept for the worker pattern.
//!
//! Demonstrates:
//! - Tokio runtime running in background thread (not main)
//! - Async operations (sleep) without blocking UI
//! - mpsc communication between threads
//!
//! This worker simulates async work by echoing messages with a delay.

use super::{WorkerCommand, WorkerResponse};
use std::sync::mpsc::{Receiver, Sender};

/// Echo worker function that processes commands in a background thread.
///
/// This worker:
/// 1. Creates a tokio runtime in the worker thread (NOT the main UI thread)
/// 2. Sends a Ready signal to indicate initialization complete
/// 3. Enters an event loop processing commands until Shutdown
///
/// # Arguments
///
/// * `cmd_rx` - Receiver for commands from the UI thread
/// * `resp_tx` - Sender for responses back to the UI thread
///
/// # Example
///
/// ```rust,ignore
/// use crate::workers::{WorkerHandle, echo_worker};
///
/// let handle = WorkerHandle::spawn(echo_worker);
/// ```
pub fn echo_worker(cmd_rx: Receiver<WorkerCommand>, resp_tx: Sender<WorkerResponse>) {
    // CRITICAL: Create tokio runtime in THIS worker thread, NOT the main thread.
    // egui runs on the main thread and must never be blocked by async operations.
    let rt = match tokio::runtime::Runtime::new() {
        Ok(runtime) => runtime,
        Err(e) => {
            eprintln!("Failed to create tokio runtime in echo worker: {}", e);
            return;
        }
    };

    // Signal that worker is initialized and ready
    if resp_tx.send(WorkerResponse::Ready).is_err() {
        eprintln!("Failed to send Ready signal - receiver dropped");
        return;
    }

    // Event loop: process commands until Shutdown or channel closed
    loop {
        match cmd_rx.recv() {
            Ok(WorkerCommand::Echo(msg)) => {
                // Simulate async work using tokio runtime
                // This demonstrates that async operations run in background without blocking UI
                let response = rt.block_on(async {
                    // Simulate I/O or computation delay
                    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

                    // Process the message
                    format!("Echo: {}", msg)
                });

                // Send response back to UI thread
                if resp_tx.send(WorkerResponse::Echo(response)).is_err() {
                    // Receiver dropped, exit gracefully
                    break;
                }
            }

            Ok(WorkerCommand::Shutdown) => {
                // Graceful shutdown requested
                break;
            }

            Err(_) => {
                // Channel closed (sender dropped), exit gracefully
                break;
            }
        }
    }

    // Worker thread exits here
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workers::WorkerHandle;

    #[test]
    fn test_echo_worker_responds() {
        let handle = WorkerHandle::spawn(echo_worker);

        // Wait for Ready signal
        let ready = handle
            .response_rx
            .recv_timeout(std::time::Duration::from_secs(1));
        assert!(matches!(ready, Ok(WorkerResponse::Ready)));

        // Send echo command
        handle
            .send_command(WorkerCommand::Echo("test".to_string()))
            .unwrap();

        // Receive echo response
        let response = handle
            .response_rx
            .recv_timeout(std::time::Duration::from_secs(1));
        assert!(matches!(response, Ok(WorkerResponse::Echo(msg)) if msg == "Echo: test"));

        // Shutdown
        handle.shutdown();
    }

    #[test]
    fn test_echo_worker_shutdown() {
        let handle = WorkerHandle::spawn(echo_worker);

        // Wait for Ready
        handle
            .response_rx
            .recv_timeout(std::time::Duration::from_secs(1))
            .unwrap();

        // Send shutdown
        handle.shutdown();

        // Worker should terminate, subsequent sends should fail
        std::thread::sleep(std::time::Duration::from_millis(200));
        let result = handle.send_command(WorkerCommand::Echo("test".to_string()));

        // Channel should be disconnected after shutdown
        assert!(result.is_err());
    }
}
