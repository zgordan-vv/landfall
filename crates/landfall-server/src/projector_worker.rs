//! Worker loop for durable `project_trace` jobs.

use std::future::Future;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

/// Minimal job payload delivered by the queue adapter.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct ProjectTraceJob {
    pub trace_id: Uuid,
}

/// Runs projection jobs until cancelled or the queue is closed.
pub async fn run_project_trace_worker<F, Fut>(
    mut receiver: mpsc::Receiver<ProjectTraceJob>,
    cancellation: CancellationToken,
    mut process: F,
) where
    F: FnMut(ProjectTraceJob) -> Fut,
    Fut: Future<Output = ()>,
{
    loop {
        tokio::select! {
            () = cancellation.cancelled() => break,
            job = receiver.recv() => match job {
                Some(job) => process(job).await,
                None => break,
            },
        }
    }
}

/// Creates a bounded projection queue, making backpressure explicit.
#[must_use]
pub fn project_trace_queue(
    capacity: usize,
) -> (
    mpsc::Sender<ProjectTraceJob>,
    mpsc::Receiver<ProjectTraceJob>,
) {
    mpsc::channel(capacity)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::{ProjectTraceJob, project_trace_queue, run_project_trace_worker};
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };
    use tokio_util::sync::CancellationToken;
    use uuid::Uuid;

    #[tokio::test]
    async fn worker_processes_jobs_and_stops_on_cancellation() {
        let (sender, receiver) = project_trace_queue(2);
        let processed = Arc::new(AtomicUsize::new(0));
        let marker = Arc::clone(&processed);
        let token = CancellationToken::new();
        let worker_token = token.clone();
        let handle = tokio::spawn(run_project_trace_worker(
            receiver,
            worker_token,
            move |_job| {
                let marker = Arc::clone(&marker);
                async move {
                    marker.fetch_add(1, Ordering::SeqCst);
                }
            },
        ));
        sender
            .send(ProjectTraceJob {
                trace_id: Uuid::now_v7(),
            })
            .await
            .unwrap();
        while processed.load(Ordering::SeqCst) == 0 {
            tokio::task::yield_now().await;
        }
        token.cancel();
        handle.await.unwrap();
        assert_eq!(processed.load(Ordering::SeqCst), 1);
    }
}
