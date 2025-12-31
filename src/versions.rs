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

pub fn get_current_version_from_toml() -> Result<Option<Version>, Box<dyn std::error::Error>> {
    let path = Path::new("./current_version.toml");
    if !path.exists() {
        return Ok(None);
    }

    let content = fs::read_to_string(path)?;
    let release: Release = toml::from_str(&content)?;
    Ok(Some(release.current_version))
}

pub fn set_current_version(module_dir: &Path, version: &Version) -> Result<(), std::io::Error> {
    let version_file_path = module_dir.join(".version");
    let mut file = File::create(version_file_path)?;
    file.write_all(version.to_string().as_bytes())?;
    Ok(())
}

pub fn should_install(new_version_str: &str) -> Result<bool, Box<dyn std::error::Error>> {
    let new_version =
        parse_version(new_version_str).map_err(|e| format!("Invalid new version string: {}", e))?;

    if let Some(current_version) = get_current_version_from_toml()? {
        if new_version <= current_version {
            return Ok(false);
        }
    }
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::tempdir;

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
        let dir = tempdir()?;
        let file_path = dir.path().join("current_version.toml");

        let mut file = File::create(&file_path)?;
        file.write_all(b"current_version = \"1.2.3\"\ninstalled = \"sometime\"")?;

        // Change current working directory to the temp directory
        let original_dir = std::env::current_dir()?;
        std::env::set_current_dir(&dir)?;

        let current_version = get_current_version_from_toml()?;
        assert!(current_version.is_some());
        assert_eq!(current_version.unwrap().to_string(), "1.2.3");

        // Restore original working directory
        std::env::set_current_dir(&original_dir)?;
        Ok(())
    }

    #[test]
    fn test_should_install() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempdir()?;
        let file_path = dir.path().join("current_version.toml");

        // Test case 1: New version is higher
        let mut file = File::create(&file_path)?;
        file.write_all(b"current_version = \"1.0.0\"\ninstalled = \"sometime\"")?;
        let original_dir = std::env::current_dir()?;
        std::env::set_current_dir(&dir)?;
        assert_eq!(should_install("1.1.0")?, true);
        std::env::set_current_dir(&original_dir)?;
        fs::remove_file(&file_path)?;

        // Test case 2: New version is lower or equal
        let mut file = File::create(&file_path)?;
        file.write_all(b"current_version = \"1.2.0\"\ninstalled = \"sometime\"")?;
        std::env::set_current_dir(&dir)?;
        assert_eq!(should_install("1.1.0")?, false);
        assert_eq!(should_install("1.2.0")?, false);
        std::env::set_current_dir(&original_dir)?;
        fs::remove_file(&file_path)?;

        // Test case 3: No current_version.toml
        std::env::set_current_dir(&dir)?;
        assert_eq!(should_install("1.0.0")?, true);
        std::env::set_current_dir(&original_dir)?;

        Ok(())
    }
}
