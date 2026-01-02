use serde::Deserialize;
use std::fmt::{self, Display};

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
    pub binary: String,
    pub checksum: String,
    pub target_dir: String,
    pub start_command: Option<String>,
    pub rollback_keep: Option<usize>,
}

impl Display for Manifest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Fetched manifest for version set {}", self.version_set)?;
        writeln!(f, "Modules: ")?;
        for m in &self.modules {
            writeln!(
                f,
                "  - {} v{} (target: {})",
                m.name, m.version, m.target_dir
            )?;
        }
        Ok(())
    }
}
