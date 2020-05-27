use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct Target {
    #[serde(default)]
    pub dependencies: Vec<String>,
    #[serde(default)]
    pub input_paths: Vec<String>,
    #[serde(default)]
    pub output_paths: Vec<String>,
    #[serde(default)]
    pub build: Option<String>,
    #[serde(default)]
    pub service: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct Project {
    #[serde(default)]
    pub imports: Vec<String>,
    #[serde(default)]
    pub targets: HashMap<String, Target>,
}
