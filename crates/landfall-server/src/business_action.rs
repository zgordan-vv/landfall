use serde::Serialize;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct BusinessActionDetail {
    pub business_action_id: String,
    pub name: Option<String>,
    pub trace_ids: Vec<String>,
    pub successful_traces: usize,
    pub multiple_success_warning: bool,
}

pub fn build_business_action_detail(
    id: impl Into<String>,
    name: Option<String>,
    trace_ids: Vec<String>,
    successful_traces: usize,
) -> BusinessActionDetail {
    BusinessActionDetail {
        business_action_id: id.into(),
        name,
        trace_ids,
        successful_traces,
        multiple_success_warning: successful_traces > 1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn multiple_successes_are_explicitly_flagged() {
        let detail = build_business_action_detail(
            "action-1",
            Some("checkout".into()),
            vec!["trace-1".into(), "trace-2".into()],
            2,
        );
        assert!(detail.multiple_success_warning);
        assert_eq!(detail.trace_ids.len(), 2);
        assert!(
            !build_business_action_detail("action-2", None, vec!["trace".into()], 1)
                .multiple_success_warning
        );
    }
}
