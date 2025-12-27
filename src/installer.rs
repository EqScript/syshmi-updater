use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use tar::Archive;
use flate2::read::GzDecoder;
use crate::manifest::Manifest;
use sha2::{Sha256, Digest};

fn verify_checksum(file_path: &Path, expected_checksum: &str) -> Result<(), String> {
    println!("Verifying checksum for {:?}", file_path);
    let mut file = File::open(file_path).map_err(|e| format!("Failed to open file {:?}: {}", file_path, e))?;
    let mut hasher = Sha256::new();
    
    let mut buffer = [0; 8192];
    loop {
        let n = file.read(&mut buffer).map_err(|e| format!("Failed to read file chunk: {}", e))?;
        if n == 0 {
            break;
        }
        hasher.update(&buffer[..n]);
    }

    let hash = hasher.finalize();
    let hash_hex = format!("{:x}", hash);

    if hash_hex.eq_ignore_ascii_case(expected_checksum) {
        println!("Checksum for {:?} is valid.", file_path.file_name().unwrap_or_default());
        Ok(())
    } else {
        Err(format!("Checksum mismatch for {:?}!\n  Expected: {}\n  Got:      {}", file_path.file_name().unwrap_or_default(), expected_checksum, hash_hex))
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

pub fn install(archive_path: &str, staging_dir: &str, manifest: &Manifest) -> Result<(), Box<dyn std::error::Error>> {
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

    // Create a single timestamped archive directory for this update operation
    let staging_parent = Path::new(staging_dir).parent().ok_or("Invalid staging dir")?;
    let archive_base_dir = staging_parent.join("archive");

    for module in &manifest.modules {
        println!("Processing module: {}", module.name);

        let module_binary_name = Path::new(module.start_command.as_deref().unwrap_or_default())
            .file_name()
            .and_then(|s| s.to_str())
            .ok_or(format!("Module {} has invalid start_command", module.name))?;

        let found_file_path = find_file_in_dir(&unpack_dir, module_binary_name)
            .ok_or(format!("Could not find module binary '{}' in unpacked archive.", module_binary_name))?;

        verify_checksum(&found_file_path, &module.checksum)?;

        let target_file_path = Path::new(&module.target_dir).join(module_binary_name);
        
        // Backup existing file if it exists
        if target_file_path.exists() {
            let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
            let archive_op_dir = archive_base_dir.join(format!("{}_{}", module.name, timestamp));
            fs::create_dir_all(&archive_op_dir)?;
            let backup_path = archive_op_dir.join(module_binary_name);
            println!("  - Backing up {:?} to {:?}", target_file_path, backup_path);
            fs::rename(&target_file_path, backup_path)?;
        }
        
        // Place new module file
        println!("  - Installing {:?} to {:?}", found_file_path, target_file_path);
        fs::create_dir_all(target_file_path.parent().unwrap())?;
        fs::rename(found_file_path, &target_file_path)?;
    }

    // Cleanup
    fs::remove_dir_all(&unpack_dir)?;
    println!("Installation complete. Cleaned up temporary unpack directory.");
    
    Ok(())
}
