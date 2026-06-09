use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[cfg(feature = "schema")]
use schemars::JsonSchema;
#[cfg(feature = "ts-export")]
use ts_rs::TS;
#[cfg(feature = "openapi")]
use utoipa::ToSchema;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct ProvenanceMap(
    #[cfg_attr(feature = "ts-export", ts(type = "Record<string, Array<string>>"))]
    pub  BTreeMap<String, Vec<String>>,
);

impl ProvenanceMap {
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn insert(&mut self, path: impl Into<String>, source_ids: Vec<String>) {
        self.0.insert(path.into(), source_ids);
    }
}
