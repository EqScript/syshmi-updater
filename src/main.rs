use serde::Deserialize;
use std::fs;
use std::fmt;


#[derive(Debug, Deserialize)]
struct Config {
    endpoint: String,
    auth_file: String,
    staging_dir: String,
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



impl fmt::Display for Config {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "endpoint       = {}", self.endpoint)?;
        writeln!(f, "auth_file      = {}", self.auth_file)?;
        writeln!(f, "staging_dir    = {}", self.staging_dir)?;
        writeln!(f, "rollback_keep  = {}", self.rollback_keep)?;
        writeln!(f, "update_interval= {}", self.update_interval)?;
        writeln!(f, "self_update    = {}", self.self_update)?;
        writeln!(f, "log_file       = {}", self.log_file)?;
        Ok(())
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
                println!("Loaded and verified primary config:\n{}", cfg);
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
                        println!("Loaded and verified fallback config:\n{}", cfg);
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
