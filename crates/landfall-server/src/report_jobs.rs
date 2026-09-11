//! Bounded report-job API model and asynchronous worker.

use std::{
    collections::HashMap,
    future::Future,
    sync::{Arc, Mutex},
};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum ReportJobStatus {
    Queued,
    Running,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ReportJob {
    pub id: Uuid,
    pub projection_watermark: u64,
    pub status: ReportJobStatus,
    pub error: Option<String>,
}

#[derive(Clone, Default)]
pub struct ReportJobRegistry {
    jobs: Arc<Mutex<HashMap<Uuid, ReportJob>>>,
}

impl ReportJobRegistry {
    pub fn create(&self, projection_watermark: u64) -> ReportJob {
        let job = ReportJob {
            id: Uuid::now_v7(),
            projection_watermark,
            status: ReportJobStatus::Queued,
            error: None,
        };
        self.jobs
            .lock()
            .expect("registry lock")
            .insert(job.id, job.clone());
        job
    }
    pub fn get(&self, id: Uuid) -> Option<ReportJob> {
        self.jobs.lock().ok()?.get(&id).cloned()
    }
    fn set_status(&self, id: Uuid, status: ReportJobStatus, error: Option<String>) {
        if let Ok(mut jobs) = self.jobs.lock() {
            if let Some(job) = jobs.get_mut(&id) {
                job.status = status;
                job.error = error;
            }
        }
    }
}

pub fn report_job_queue(capacity: usize) -> (mpsc::Sender<ReportJob>, mpsc::Receiver<ReportJob>) {
    mpsc::channel(capacity)
}

pub async fn run_report_worker<F, Fut>(
    mut receiver: mpsc::Receiver<ReportJob>,
    registry: ReportJobRegistry,
    cancellation: CancellationToken,
    mut process: F,
) where
    F: FnMut(ReportJob) -> Fut,
    Fut: Future<Output = Result<(), String>>,
{
    loop {
        tokio::select! { () = cancellation.cancelled() => break, job = receiver.recv() => match job { Some(job) => { registry.set_status(job.id, ReportJobStatus::Running, None); match process(job.clone()).await { Ok(()) => registry.set_status(job.id, ReportJobStatus::Completed, None), Err(error) => registry.set_status(job.id, ReportJobStatus::Failed, Some(error)), } }, None => break } }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn worker_transitions_job_to_completed() {
        let registry = ReportJobRegistry::default();
        let job = registry.create(42);
        let (tx, rx) = report_job_queue(1);
        let cancel = CancellationToken::new();
        let worker_cancel = cancel.clone();
        let worker_registry = registry.clone();
        let handle = tokio::spawn(run_report_worker(
            rx,
            worker_registry,
            worker_cancel,
            |_job| async { Ok(()) },
        ));
        tx.send(job.clone()).await.unwrap();
        for _ in 0..20 {
            if registry.get(job.id).unwrap().status == ReportJobStatus::Completed {
                break;
            }
            tokio::task::yield_now().await;
        }
        cancel.cancel();
        handle.await.unwrap();
        assert_eq!(
            registry.get(job.id).unwrap().status,
            ReportJobStatus::Completed
        );
    }
}
