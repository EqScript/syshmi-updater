use crate::config::Config;
use log::info;
use semver::Version;
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::Write;
use std::path::Path;

#[derive(Debug, Serialize, Deserialize)]
pub struct Release {
    pub current_version: Version,
    pub installed: String,
}

pub fn parse_version(version_str: &str) -> Result<Version, semver::Error> {
    // Try to parse directly first
    if let Ok(version) = Version::parse(version_str) {
        return Ok(version);
    }

    // If direct parsing fails, apply replacements for common shorthands
    let replaced_version_str = version_str
        .replace("a", "-alpha")
        .replace("b", "-beta")
        .replace("rc", "-rc");
    Version::parse(&replaced_version_str)
}

pub fn get_current_version_from_toml(
    config: &Config,
) -> Result<Option<Version>, Box<dyn std::error::Error>> {
    let base_dir = Path::new(&config.staging_dir)
        .parent()
        .ok_or("Invalid staging dir")?;
    let path = base_dir.join("current_version.toml");
    if !path.exists() {
        return Ok(None);
    }

    let content = fs::read_to_string(path)?;
    let table: toml::Value = toml::from_str(&content)?;

    if let Some(version_str) = table.get("current_version").and_then(|v| v.as_str()) {
        match parse_version(version_str) {
            Ok(version) => Ok(Some(version)),
            Err(_) => Ok(None), // Or handle the error more gracefully
        }
    } else {
        Ok(None)
    }
}

pub fn set_current_version(module_dir: &Path, version: &Version) -> Result<(), std::io::Error> {
    let version_file_path = module_dir.join(".version");
    let mut file = File::create(version_file_path)?;
    file.write_all(version.to_string().as_bytes())?;
    Ok(())
}

pub fn should_install(
    new_version_str: &str,
    config: &Config,
) -> Result<bool, Box<dyn std::error::Error>> {
    let new_version =
        parse_version(new_version_str).map_err(|e| format!("Invalid new version string: {}", e))?;

    info!("Checking if should install new version: {}", new_version);

    if let Some(current_version) = get_current_version_from_toml(config)? {
        info!("Found current version: {}", current_version);
        if new_version <= current_version {
            info!("New version is not greater than current version. Skipping installation.");
            return Ok(false);
        }
    } else {
        info!("No current version found. Proceeding with installation.");
    }
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;
    use tempfile::tempdir;

    fn setup_test_env() -> (tempfile::TempDir, Config) {
        let dir = tempdir().unwrap();
        let staging_dir = dir.path().join("staging");
        fs::create_dir(&staging_dir).unwrap();
        let config = Config {
            endpoint: "dummy".to_string(),
            auth_file: "dummy".to_string(),
            staging_dir: staging_dir.to_str().unwrap().to_string(),
            rollback_keep: 2,
            update_interval: "daily".to_string(),
            self_update: false,
            log_file: "dummy".to_string(),
        };
        (dir, config)
    }

    #[test]
    fn test_parse_version() {
        assert!(parse_version("0.0.1a").is_ok());
        assert!(parse_version("0.0.1b").is_ok());
        assert!(parse_version("0.0.1rc").is_ok());
        assert!(parse_version("0.1.0").is_ok());
        assert!(parse_version("1.0.0").is_ok());
        assert!(parse_version("1.0.0-alpha").is_ok());
        assert!(parse_version("1.0.0-beta").is_ok());
        assert!(parse_version("1.0.0-rc.1").is_ok());
    }

    #[test]
    fn test_invalid_version() {
        assert!(parse_version("invalid").is_err());
    }

    #[test]
    fn test_get_current_version_from_toml() -> Result<(), Box<dyn std::error::Error>> {
        let (dir, config) = setup_test_env();
        let file_path = dir.path().join("current_version.toml");

        let mut file = File::create(&file_path)?;
        file.write_all(b"current_version = \"1.2.3\"\ninstalled = \"sometime\"")?;

        let current_version = get_current_version_from_toml(&config)?;
        assert!(current_version.is_some());
        assert_eq!(current_version.unwrap().to_string(), "1.2.3");

        Ok(())
    }

    #[test]
    fn test_should_install() -> Result<(), Box<dyn std::error::Error>> {
        let (dir, config) = setup_test_env();
        let file_path = dir.path().join("current_version.toml");

        // Test case 1: New version is higher
        let mut file = File::create(&file_path)?;
        file.write_all(b"current_version = \"1.0.0\"\ninstalled = \"sometime\"")?;
        assert_eq!(should_install("1.1.0", &config)?, true);
        fs::remove_file(&file_path)?;

        // Test case 2: New version is lower or equal
        let mut file = File::create(&file_path)?;
        file.write_all(b"current_version = \"1.2.0\"\ninstalled = \"sometime\"")?;
        assert_eq!(should_install("1.1.0", &config)?, false);
        assert_eq!(should_install("1.2.0", &config)?, false);
        fs::remove_file(&file_path)?;

        // Test case 3: No current_version.toml
        assert_eq!(should_install("1.0.0", &config)?, true);

        // Test case 4: version with 'b'
        let mut file = File::create(&file_path)?;
        file.write_all(b"current_version = \"0.0.1b\"\ninstalled = \"sometime\"")?;
        assert_eq!(should_install("0.0.1-beta", &config)?, false);
        fs::remove_file(&file_path)?;

        Ok(())
    }
}
