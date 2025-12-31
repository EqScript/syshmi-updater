use semver::Version;
use std::path::Path;
use std::fs::{self, File};
use std::io::Write;

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

pub fn get_current_version(module_dir: &Path) -> Option<Version> {
    let version_file_path = module_dir.join(".version");
    if !version_file_path.exists() {
        return None;
    }

    fs::read_to_string(version_file_path)
        .ok()
        .and_then(|v| parse_version(&v).ok())
}

pub fn set_current_version(module_dir: &Path, version: &Version) -> Result<(), std::io::Error> {
    let version_file_path = module_dir.join(".version");
    let mut file = File::create(version_file_path)?;
    file.write_all(version.to_string().as_bytes())?;
    Ok(())
}

pub fn should_install(new_version_str: &str, module_dir: &Path) -> Result<Option<Version>, Box<dyn std::error::Error>> {
    let new_version = parse_version(new_version_str)
        .map_err(|e| format!("Invalid new version string: {}", e))?;

    if let Some(current_version) = get_current_version(module_dir) {
        if new_version <= current_version {
            println!(
                "  - Module is up to date (version {}). Skipping.",
                current_version
            );
            return Ok(None);
        }
    }
    Ok(Some(new_version))
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
