//! Durable job claim/lease recovery semantics for benchmarks.

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum JobState {
    Pending,
    Running,
    Completed,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct DurableJob {
    pub id: String,
    pub state: JobState,
    pub attempts: u32,
    pub lease_until: Option<u64>,
}

impl DurableJob {
    /// Claims a pending or expired job and assigns a lease.
    pub fn claim(&mut self, now: u64, lease_seconds: u64) -> bool {
        let available =
            self.state == JobState::Pending || self.lease_until.is_some_and(|until| until <= now);
        if !available || lease_seconds == 0 {
            return false;
        }
        self.state = JobState::Running;
        self.attempts = self.attempts.saturating_add(1);
        self.lease_until = Some(now.saturating_add(lease_seconds));
        true
    }
    pub fn complete(&mut self) {
        if self.state == JobState::Running {
            self.state = JobState::Completed;
            self.lease_until = None;
        }
    }
    #[must_use]
    pub fn recover_expired(&mut self, now: u64) -> bool {
        if self.state == JobState::Running && self.lease_until.is_some_and(|until| until <= now) {
            self.state = JobState::Pending;
            self.lease_until = None;
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn expired_lease_is_recovered_and_reclaimed() {
        let mut job = DurableJob {
            id: "job-1".into(),
            state: JobState::Pending,
            attempts: 0,
            lease_until: None,
        };
        assert!(job.claim(100, 10));
        assert!(!job.claim(105, 10));
        assert!(job.recover_expired(110));
        assert!(job.claim(110, 10));
        assert_eq!(job.attempts, 2);
        job.complete();
        assert_eq!(job.state, JobState::Completed);
        assert!(!job.recover_expired(999));
    }
}
