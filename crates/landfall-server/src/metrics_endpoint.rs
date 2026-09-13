//! Prometheus exposition for durable Landfall operational state.

use sqlx::{PgPool, Row};

/// Values sampled from PostgreSQL once per scrape.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct DurableMetrics {
    pub projects: i64,
    pub environments: i64,
    pub enabled_routes: i64,
    pub ready_jobs: i64,
    pub running_jobs: i64,
    pub dead_letter_jobs: i64,
    pub events_last_24h: i64,
}

/// Loads non-tenant-specific operational gauges from the durable store.
pub async fn load(pool: &PgPool) -> Result<DurableMetrics, sqlx::Error> {
    let row = sqlx::query("SELECT (SELECT COUNT(*)::bigint FROM control.projects) AS projects, (SELECT COUNT(*)::bigint FROM control.environments) AS environments, (SELECT COUNT(*)::bigint FROM control.routes WHERE enabled) AS enabled_routes, (SELECT COUNT(*)::bigint FROM work.jobs WHERE status = 'ready') AS ready_jobs, (SELECT COUNT(*)::bigint FROM work.jobs WHERE status = 'running') AS running_jobs, (SELECT COUNT(*)::bigint FROM work.jobs WHERE status = 'dead_letter') AS dead_letter_jobs, (SELECT COUNT(*)::bigint FROM telemetry.raw_events WHERE received_at >= now() - interval '24 hours') AS events_last_24h")
        .fetch_one(pool)
        .await?;
    Ok(DurableMetrics {
        projects: row.get("projects"),
        environments: row.get("environments"),
        enabled_routes: row.get("enabled_routes"),
        ready_jobs: row.get("ready_jobs"),
        running_jobs: row.get("running_jobs"),
        dead_letter_jobs: row.get("dead_letter_jobs"),
        events_last_24h: row.get("events_last_24h"),
    })
}

/// Renders the Prometheus text exposition without customer or credential data.
#[must_use]
pub fn render(ready: bool, durable: Option<DurableMetrics>) -> String {
    let database_up = i64::from(durable.is_some());
    let process_ready = i64::from(ready && durable.is_some());
    let mut output = String::from(
        "# HELP landfall_build_info Immutable build identity.\n# TYPE landfall_build_info gauge\nlandfall_build_info{version=\"0.1.0\"} 1\n# HELP landfall_database_up Whether the server can query PostgreSQL.\n# TYPE landfall_database_up gauge\n",
    );
    output.push_str(&format!("landfall_database_up {database_up}\n"));
    output.push_str("# HELP landfall_process_ready Whether this server instance is ready.\n# TYPE landfall_process_ready gauge\n");
    output.push_str(&format!("landfall_process_ready {process_ready}\n"));
    if let Some(values) = durable {
        output.push_str(
            "# HELP landfall_projects Total configured projects.\n# TYPE landfall_projects gauge\n",
        );
        output.push_str(&format!("landfall_projects {}\n", values.projects));
        output.push_str("# HELP landfall_environments Total configured environments.\n# TYPE landfall_environments gauge\n");
        output.push_str(&format!("landfall_environments {}\n", values.environments));
        output.push_str("# HELP landfall_enabled_rpc_routes Total enabled observation routes.\n# TYPE landfall_enabled_rpc_routes gauge\n");
        output.push_str(&format!(
            "landfall_enabled_rpc_routes {}\n",
            values.enabled_routes
        ));
        output.push_str(
            "# HELP landfall_jobs Current durable jobs by state.\n# TYPE landfall_jobs gauge\n",
        );
        output.push_str(&format!("landfall_jobs{{state=\"ready\"}} {}\nlandfall_jobs{{state=\"running\"}} {}\nlandfall_jobs{{state=\"dead_letter\"}} {}\n", values.ready_jobs, values.running_jobs, values.dead_letter_jobs));
        output.push_str("# HELP landfall_events_received_24h Events durably received in the trailing 24 hours.\n# TYPE landfall_events_received_24h gauge\n");
        output.push_str(&format!(
            "landfall_events_received_24h {}\n",
            values.events_last_24h
        ));
    }
    output
}

#[cfg(test)]
mod tests {
    use super::{DurableMetrics, render};

    #[test]
    fn metrics_are_prometheus_text_and_omit_customer_values() {
        let text = render(
            true,
            Some(DurableMetrics {
                projects: 1,
                environments: 2,
                enabled_routes: 3,
                ready_jobs: 4,
                running_jobs: 5,
                dead_letter_jobs: 6,
                events_last_24h: 7,
            }),
        );
        assert!(text.contains("# TYPE landfall_jobs gauge"));
        assert!(text.contains("landfall_jobs{state=\"dead_letter\"} 6"));
        assert!(text.contains("landfall_process_ready 1"));
        assert!(!text.contains("endpoint"));
        assert!(!text.contains("token"));
    }

    #[test]
    fn unavailable_database_is_explicit() {
        let text = render(true, None);
        assert!(text.contains("landfall_database_up 0"));
        assert!(text.contains("landfall_process_ready 0"));
    }
}
