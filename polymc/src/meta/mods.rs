
use crate::meta::manifest::{Manifest, Requirement, OS};
use serde::{Deserialize, Serialize};
use serde_json::{self, Value};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ModLoader {
    Fabric {
        // Rename from "name"
        #[serde(rename = "name")]
        maven_id: String,
        #[serde(rename = "url")]
        base_url: String,
    }
}