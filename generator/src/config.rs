use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub scanner_name: Option<String>,
    pub output_file: Option<String>,
    pub template_file: Option<String>,
    pub templates: Option<HashMap<String, String>>,
    pub patterns: Vec<PatternDef>,
    #[serde(default)]
    pub skip_regexes: Vec<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct PatternDef {
    pub token: String,
    pub regex: String,
}
