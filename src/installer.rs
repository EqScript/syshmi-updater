use std::fs::{self, File};
use std::io::{Read, Write};
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use flate2::read::GzDecoder;
use sha2::{Digest, Sha256};
use tar::Archive;
use toml;

use crate::manifest::Manifest;
use crate::versions::{self, Release};

fn verify_checksum(file_path: &Path, expected_checksum: &str) -> Result<(), String> {
    println!("Verifying checksum for {:?}", file_path);
    let mut file =
        File::open(file_path).map_err(|e| format!("Failed to open file {:?}: {}", file_path, e))?;
    let mut hasher = Sha256::new();

    let mut buffer = [0; 8192];
    loop {
        let n = file
            .read(&mut buffer)
            .map_err(|e| format!("Failed to read file chunk: {}", e))?;
        if n == 0 {
            break;
        }
        hasher.update(&buffer[..n]);
    }

    let hash = hasher.finalize();
    let hash_hex = format!("{:x}", hash);

    if hash_hex.eq_ignore_ascii_case(expected_checksum) {
        println!(
            "Checksum for {:?} is valid.",
            file_path.file_name().unwrap_or_default()
        );
        Ok(())
    } else {
        Err(format!(
            "Checksum mismatch for {:?}\n  Expected: {}\n  Got:      {}",
            file_path.file_name().unwrap_or_default(),
            expected_checksum,
            hash_hex
        ))
    }
}

fn find_file_in_dir(dir: &Path, file_name: &str) -> Option<PathBuf> {
    if !dir.is_dir() {
        return None;
    }
    for entry in fs::read_dir(dir).ok()? {
        let entry = entry.ok()?;
        let path = entry.path();
        if path.is_dir() {
            if let Some(found_path) = find_file_in_dir(&path, file_name) {
                return Some(found_path);
            }
        } else if path.file_name().and_then(|s| s.to_str()) == Some(file_name) {
            return Some(path);
        }
    }
    None
}

pub fn install(
    archive_path: &str,
    staging_dir: &str,
    manifest: &Manifest,
) -> Result<(), Box<dyn std::error::Error>> {
    let staging_parent = Path::new(staging_dir)
        .parent()
        .ok_or("Invalid staging dir")?;
    let current_version_toml_path = staging_parent.join("current_version.toml");

    let unpack_dir = Path::new(staging_dir).join("unpacked");
    if unpack_dir.exists() {
        fs::remove_dir_all(&unpack_dir)?;
    }
    fs::create_dir_all(&unpack_dir)?;

    println!("Unpacking archive to temporary directory: {:?}", unpack_dir);
    let tar_gz = File::open(archive_path)?;
    let tar = GzDecoder::new(tar_gz);
    let mut archive = Archive::new(tar);
    archive.unpack(&unpack_dir)?;

    let _archive_base_dir = staging_parent.join("archive");

    let mut at_least_one_module_installed = false;

    for module in &manifest.modules {
        println!("Processing module: {}", module.name);

        let module_dir = Path::new(&module.target_dir);
        // The main.rs already decided if an update is needed at a global level (for the manifest.version_set)
        // so installer.rs should not re-evaluate whether individual modules should be installed.
        // It should simply proceed with installing all modules described in the manifest.

        at_least_one_module_installed = true;

        let module_binary_name = &module.binary;

        let found_file_path = find_file_in_dir(&unpack_dir, module_binary_name).ok_or(format!(
            "Could not find module binary '{}' in unpacked archive.",
            module_binary_name
        ))?;

        verify_checksum(&found_file_path, &module.checksum)?;

        let release_dir = module_dir
            .join("releases")
            .join(format!("v{}", &module.version));

        if release_dir.exists() {
            fs::remove_dir_all(&release_dir)?;
        }
        fs::create_dir_all(&release_dir)?;

        let target_file_path = release_dir.join(module_binary_name);

        println!(
            "  - Installing {:?} to {:?}",
            found_file_path, target_file_path
        );
        fs::rename(found_file_path, &target_file_path)?;

        // Using start_command from the manifest for the target binary name
        let active_binary_path = Path::new(module.start_command.as_deref().unwrap());
        if active_binary_path.exists() {
            let _ = fs::remove_file(&active_binary_path);
        }

        println!(
            "  - Activating new version by symlinking {:?} to {:?}",
            target_file_path, active_binary_path
        );
        match symlink(&target_file_path, &active_binary_path) {
            Ok(_) => println!("  - Done"),
            Err(e) => eprintln!("Failed to create symlink: {}", e),
        }

        versions::set_current_version(module_dir, &versions::parse_version(&module.version)?)?;
        println!(
            "  - Successfully installed module {} version {}",
            module.name, module.version
        );
    }

    // Cleanup
    fs::remove_dir_all(&unpack_dir)?;

    if at_least_one_module_installed {
        let release = Release {
            current_version: versions::parse_version(&manifest.version_set)?,
            installed: SystemTime::now()
                .duration_since(UNIX_EPOCH)?
                .as_secs()
                .to_string(),
        };
        let toml_string = toml::to_string(&release)?;
        let mut file = File::create(&current_version_toml_path)?;
        file.write_all(toml_string.as_bytes())?;
        println!(
            "Installation complete. Updated current_version.toml to version {}.",
            manifest.version_set
        );
    } else {
        println!("Installation complete. No new modules were installed.");
    }

    Ok(())
}
