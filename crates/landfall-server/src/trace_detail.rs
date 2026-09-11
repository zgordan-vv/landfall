use serde::Serialize;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct TraceDetail<TAttempt, TObservation, TEvidence, TRecommendation> {
    pub trace_id: String,
    pub attempts: Vec<TAttempt>,
    pub observations: Vec<TObservation>,
    pub evidence: Vec<TEvidence>,
    pub recommendations: Vec<TRecommendation>,
    pub projection_watermark: u64,
}

impl<TAttempt, TObservation, TEvidence, TRecommendation>
    TraceDetail<TAttempt, TObservation, TEvidence, TRecommendation>
{
    pub fn new(trace_id: impl Into<String>, projection_watermark: u64) -> Self {
        Self {
            trace_id: trace_id.into(),
            attempts: Vec::new(),
            observations: Vec::new(),
            evidence: Vec::new(),
            recommendations: Vec::new(),
            projection_watermark,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn trace_detail_keeps_all_read_model_sections() {
        let mut detail = TraceDetail::<String, String, String, String>::new("trace-1", 42);
        detail.attempts.push("attempt".into());
        detail.observations.push("observation".into());
        detail.evidence.push("evidence".into());
        detail.recommendations.push("recommendation".into());
        assert_eq!(detail.projection_watermark, 42);
        assert_eq!(detail.attempts.len(), 1);
        assert_eq!(detail.observations.len(), 1);
        assert_eq!(detail.evidence.len(), 1);
        assert_eq!(detail.recommendations.len(), 1);
    }
}
