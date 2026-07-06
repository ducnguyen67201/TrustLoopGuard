mod operations;

use std::collections::HashMap;
use std::sync::Arc;

use tl_core::{DEFAULT_ENVIRONMENT_ID, DEFAULT_WORKSPACE_ID};
use tl_policy::{FamilyPolicy, Policy};
use tokio::sync::RwLock;

#[derive(Debug, Clone)]
pub(super) struct MemoryPolicyRecord {
    pub(super) policy: Policy,
    pub(super) source_yaml: String,
}

#[derive(Debug, Default)]
pub struct MemoryPolicyStore {
    pub(super) inner: RwLock<HashMap<(String, String), MemoryPolicyRecord>>,
    pub(super) deployments: RwLock<HashMap<(String, String, String), bool>>,
    /// Family policies keyed by `(workspace_id, id)`.
    pub(super) families: RwLock<HashMap<(String, String), Arc<FamilyPolicy>>>,
    pub(super) family_sources: RwLock<HashMap<(String, String), String>>,
}

impl MemoryPolicyStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_policies(policies: &[Policy]) -> Self {
        let mut deployments = HashMap::new();
        let records = policies
            .iter()
            .map(|policy| {
                deployments.insert(
                    (
                        DEFAULT_WORKSPACE_ID.to_string(),
                        DEFAULT_ENVIRONMENT_ID.to_string(),
                        policy.id.clone(),
                    ),
                    true,
                );
                (
                    (DEFAULT_WORKSPACE_ID.to_string(), policy.id.clone()),
                    MemoryPolicyRecord {
                        policy: policy.clone(),
                        source_yaml: serde_yaml::to_string(policy).unwrap_or_default(),
                    },
                )
            })
            .collect();
        Self {
            inner: RwLock::new(records),
            deployments: RwLock::new(deployments),
            families: RwLock::new(HashMap::new()),
            family_sources: RwLock::new(HashMap::new()),
        }
    }
}
