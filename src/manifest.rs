use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Manifest {
    pub download_url: String,
    pub version_set: String,
    pub modules: Vec<Module>,
}

#[derive(Debug, Deserialize)]
pub struct Module {
    pub name: String,
    pub version: String,
    pub checksum: String,
    pub target_dir: String,
    pub start_command: Option<String>,
    pub rollback_keep: Option<usize>,
}
