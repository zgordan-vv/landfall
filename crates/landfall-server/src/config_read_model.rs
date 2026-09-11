//! Redacted read models for project, environment, and route configuration.

use serde::Serialize;

/// Public project configuration returned by read endpoints.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ProjectConfig {
    pub project_id: String,
    pub display_name: String,
    pub environments: Vec<EnvironmentConfig>,
}

/// Public environment configuration; secrets and raw endpoints are omitted.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct EnvironmentConfig {
    pub environment_id: String,
    pub display_name: String,
    pub privacy_mode: String,
    pub routes: Vec<RouteConfig>,
}

/// Public route configuration safe for dashboards and diagnostics.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct RouteConfig {
    pub route_id: String,
    pub label: String,
    pub enabled: bool,
    pub commitment: String,
    pub auth_configured: bool,
}

/// Creates a configuration snapshot with deterministic route ordering.
#[must_use]
pub fn project_config(
    project_id: impl Into<String>,
    display_name: impl Into<String>,
    mut environments: Vec<EnvironmentConfig>,
) -> ProjectConfig {
    environments.sort_by(|left, right| left.environment_id.cmp(&right.environment_id));
    for environment in &mut environments {
        environment
            .routes
            .sort_by(|left, right| left.route_id.cmp(&right.route_id));
    }
    ProjectConfig {
        project_id: project_id.into(),
        display_name: display_name.into(),
        environments,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshot_is_sorted_and_redacted() {
        let snapshot = project_config(
            "project-1",
            "Demo",
            vec![EnvironmentConfig {
                environment_id: "prod".into(),
                display_name: "Production".into(),
                privacy_mode: "strict".into(),
                routes: vec![
                    RouteConfig {
                        route_id: "zeta".into(),
                        label: "Zeta RPC".into(),
                        enabled: true,
                        commitment: "confirmed".into(),
                        auth_configured: true,
                    },
                    RouteConfig {
                        route_id: "alpha".into(),
                        label: "Alpha RPC".into(),
                        enabled: false,
                        commitment: "finalized".into(),
                        auth_configured: false,
                    },
                ],
            }],
        );
        assert_eq!(snapshot.environments[0].routes[0].route_id, "alpha");
        let json = serde_json::to_string(&snapshot).unwrap();
        assert!(!json.contains("endpoint"));
        assert!(!json.contains("token"));
    }
}
