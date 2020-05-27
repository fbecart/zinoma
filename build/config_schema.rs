include!("../src/config/yaml/schema.rs");

use schemars::schema_for;
use std::ffi::OsString;
use std::fs;
use std::path::Path;

pub fn generate_config_json_schema(outdir: &OsString) {
    let schema = schema_for!(Project);
    let schema_file = Path::new(outdir).join("zinoma-schema.json");
    fs::write(schema_file, serde_json::to_string_pretty(&schema).unwrap()).unwrap();
}
