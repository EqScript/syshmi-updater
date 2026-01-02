mod config;
mod installer;
mod manifest;
mod network;
mod versions;

use config::Config;
use installer::install;
use network::{download_firmware, fetch_manifest};
use versions::should_install;

use log::{info, error, LevelFilter};
use simplelog::{WriteLogger, CombinedLogger, Config as SimplelogConfig};
use std::fs::{File, create_dir_all};
use std::path::Path;

#[tokio::main]
async fn main() {
    // Trying primary path
    let cfg = Config::try_load("/srv/firmware/config.toml")
        .or_else(|_| Config::try_load("/etc/syshmi/conf.toml"))
        .unwrap_or_else(|e| {
            eprintln!("Failed to load config: {}", e);
            std::process::exit(1);
        });

    // Initialize logger
    let log_file_path = &cfg.log_file;

    // Ensure the log directory exists
    if let Some(log_dir) = Path::new(log_file_path).parent() {
        if !log_dir.exists() {
            if let Err(e) = create_dir_all(log_dir) {
                eprintln!("Failed to create log directory {}: {}", log_dir.display(), e);
                std::process::exit(1);
            }
        }
    }
    
    if let Err(e) = CombinedLogger::init(vec![
        WriteLogger::new(
            LevelFilter::Info, // Log Info level and above to file
            SimplelogConfig::default(),
            File::create(log_file_path).unwrap_or_else(|file_err| {
                eprintln!("Failed to create log file {}: {}", log_file_path, file_err);
                std::process::exit(1);
            }),
        )
    ]) {
        eprintln!("Failed to initialize logger: {}", e);
        // If logger fails to initialize, we can't log, so just exit
        std::process::exit(1);
    }

    info!("Loaded and verified config:\n{}", cfg);

    let manifest = fetch_manifest(&cfg.endpoint).await.unwrap_or_else(|e| {
        error!("Failed to fetch or parse manifest: {}", e);
        std::process::exit(1);
    });

    info!("Successfully fetched manifest:\n{}", manifest);

    // Compare versions and decide whether to install
    if !should_install(&manifest.version_set, &cfg).unwrap_or_else(|e| {
        error!("Failed to check version: {}", e);
        std::process::exit(1);
    }) {
        info!("Current version is up to date, nothing to do, now exit...");
        return;
    }

    match download_firmware(&manifest, &cfg.staging_dir).await {
        Ok(path) => {
            info!("Firmware downloaded successfully to {}", path);
            if let Err(e) = install(&path, &cfg.staging_dir, &manifest) {
                error!("Failed to install module(s): {}", e);
                std::process::exit(1);
            }
        }
        Err(e) => {
            error!("Failed to download firmware: {}", e);
            std::process::exit(1);
        }
    }
}
