mod config;
mod installer;
mod manifest;
mod network;
mod versions;

use config::Config;
use installer::install;
use network::{download_firmware, fetch_manifest};
use versions::should_install;

#[tokio::main]
async fn main() {
    // Trying primary path
    let cfg = Config::try_load("/srv/firmware/conf.toml")
        .or_else(|_| Config::try_load("/etc/syshmi/conf.toml"))
        .unwrap_or_else(|e| {
            eprintln!("{}", e);
            std::process::exit(1);
        });

    println!("Loaded and verified config:\n{}", cfg);

    let manifest = fetch_manifest(&cfg.endpoint).await.unwrap_or_else(|e| {
        eprintln!("Failed to fetch or parse manifest: {}", e);
        std::process::exit(1);
    });

    println!("Successfully fetched manifest:\n{}", manifest);

    // Compare versions and decide whether to install
    if !should_install(&manifest.version_set, &cfg).unwrap_or_else(|e| {
        eprintln!("Failed to check version: {}", e);
        std::process::exit(1);
    }) {
        println!("Current version is up to date, nothing to do, now exit...");
        return;
    }

    match download_firmware(&manifest, &cfg.staging_dir).await {
        Ok(path) => {
            println!("Firmware downloaded successfully to {}", path);
            if let Err(e) = install(&path, &cfg.staging_dir, &manifest) {
                eprintln!("Failed to install module(s): {}", e);
                std::process::exit(1);
            }
        }
        Err(e) => {
            eprintln!("Failed to download firmware: {}", e);
            std::process::exit(1);
        }
    }
}
