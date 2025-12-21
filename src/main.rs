use serde::Deserialize;
use std::fs;

#[derive(Debug, Deserialize)]
struct Config {
    endpoint: String,
    auth_file: String,
    starting_dir: String,
    rollback_keep: usize,
    update_interval: String,
    self_update: bool,
    log_file: String,
}

impl Config {
    fn check(&self) -> Result<(), String> {
        // Validating URL
        if !self.endpoint.starts_with("http") {
            return Err("Endpoint must be a valid url".into());
        }

        // Rollback policy sanity
        if self.rollback_keep == 0 {
            return Err("rollback_keep must be at least 1".into());
        }

        // Update interval sanity
        let valid_intervals = ["manual", "hourly", "daily", "daemon"];
        if !valid_intervals.contains(&self.update_interval.as_str()) {
            return Err(format!("Invalid update_interval: {}", self.update_interval));
        }

        Ok(())
    }

    fn load(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let content = fs::read_to_string(path)?;
        let config: Config = toml::from_str(&content)?;
        Ok(config)
    }
}

fn main() {
    // Trying primary path
    match Config::load("/srv/firmware/conf.toml") {
        Ok(cfg) => {
            if let Err(e) = cfg.check() {
                eprintln!("Primary config error: {}", e);
                // Decide if you want to exit or try the fallback
            } else {
                println!("Loaded and verified primary config: {:?}", cfg);
            }
        }
        Err(e) => {
            eprintln!("Failed to load primary config: {}", e);

            // Fallback to secondary path
            match Config::load("/etc/syshmi/conf.toml") {
                Ok(cfg) => {
                    if let Err(e) = cfg.check() {
                        eprintln!("Fallback config error: {}", e);
                        std::process::exit(1);
                    } else {
                        println!("Loaded and verified fallback config: {:?}", cfg);
                    }
                }
                Err(e) => {
                    eprintln!("Failed to load fallback config too: {}", e);
                    std::process::exit(1);
                }
            }
        }
    }
}
