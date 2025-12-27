mod config;
mod manifest;
mod network;
mod installer;

use config::Config;
use network::{fetch_manifest, download_firmware};
use installer::install;



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

    println!("Successfully fetched manifest:\n{:#?}", manifest);

    match download_firmware(&manifest, &cfg.staging_dir).await {
        Ok(path) => {
            println!("Firmware downloaded successfully to {}", path);
            if let Err(e) = install(&path, &cfg.staging_dir, &manifest) {
                eprintln!("Failed to install module(s): {}", e);
                std::process::exit(1);
            }
        },
        Err(e) => {
            eprintln!("Failed to download firmware: {}", e);
            std::process::exit(1);
        }
    }
}
