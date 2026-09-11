//! Cancellable background worker supervision.

use std::future::Future;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

/// Owns background tasks and coordinates graceful shutdown.
pub struct WorkerSupervisor {
    cancellation: CancellationToken,
    handles: Vec<JoinHandle<()>>,
}

impl WorkerSupervisor {
    /// Creates an empty supervisor with a shared cancellation token.
    #[must_use]
    pub fn new() -> Self {
        Self {
            cancellation: CancellationToken::new(),
            handles: Vec::new(),
        }
    }

    /// Returns a child token workers can await during their loop.
    #[must_use]
    pub fn token(&self) -> CancellationToken {
        self.cancellation.child_token()
    }

    /// Spawns and tracks one named worker future.
    pub fn spawn<F>(&mut self, _name: &'static str, worker: F)
    where
        F: Future<Output = ()> + Send + 'static,
    {
        self.handles.push(tokio::spawn(worker));
    }

    /// Cancels all workers and waits for graceful completion.
    pub async fn shutdown(self) {
        self.cancellation.cancel();
        for handle in self.handles {
            let _ = handle.await;
        }
    }
}

impl Default for WorkerSupervisor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::WorkerSupervisor;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};
    use tokio::time::{Duration, sleep};

    #[tokio::test]
    async fn shutdown_cancels_and_joins_worker() {
        let mut supervisor = WorkerSupervisor::new();
        let stopped = Arc::new(AtomicBool::new(false));
        let token = supervisor.token();
        let marker = Arc::clone(&stopped);
        supervisor.spawn("test", async move {
            token.cancelled().await;
            marker.store(true, Ordering::SeqCst);
        });
        supervisor.shutdown().await;
        sleep(Duration::from_millis(1)).await;
        assert!(stopped.load(Ordering::SeqCst));
    }
}
