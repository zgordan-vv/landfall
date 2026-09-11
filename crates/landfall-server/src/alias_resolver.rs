//! Deterministic trace alias index scoped by project and environment.

use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum AliasResolution {
    Canonical(Uuid),
    Conflict,
    Unrelated,
}

#[derive(Default)]
pub struct AliasResolver {
    index: HashMap<(Uuid, Uuid, String, String), Uuid>,
}

impl AliasResolver {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn observe(
        &mut self,
        project_id: Uuid,
        environment_id: Uuid,
        trace_id: Uuid,
        key_id: &str,
        fingerprint: &str,
    ) -> AliasResolution {
        let key = (
            project_id,
            environment_id,
            key_id.to_owned(),
            fingerprint.to_owned(),
        );
        if let Some(canonical) = self.index.get(&key) {
            return AliasResolution::Canonical(*canonical);
        }
        self.index.insert(key, trace_id);
        AliasResolution::Unrelated
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn aliases_are_scoped_and_deterministic() {
        let project = Uuid::now_v7();
        let environment = Uuid::now_v7();
        let first = Uuid::now_v7();
        let second = Uuid::now_v7();
        let mut resolver = AliasResolver::new();
        assert_eq!(
            resolver.observe(project, environment, first, "k1", "digest"),
            AliasResolution::Unrelated
        );
        assert_eq!(
            resolver.observe(project, environment, second, "k1", "digest"),
            AliasResolution::Canonical(first)
        );
        assert_eq!(
            resolver.observe(project, Uuid::now_v7(), second, "k1", "digest"),
            AliasResolution::Unrelated
        );
    }
}
