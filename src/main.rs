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

    fn try_load(path: &str) -> Result<Config, String> {
        match Config::load(path) {
            Ok(cfg) => {
                cfg.check().map_err(|e| format!("Config error in {}: {}", path, e))?;
                Ok(cfg)
            }
            Err(e) => Err(format!("Failed to load {}: {}", path, e)),
        }
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
    let cfg = Config::try_load("/srv/firmware/conf.toml")
        .or_else(|_| Config::try_load("/etc/syshmi/conf.toml"))
        .unwrap_or_else(|e| {
            eprintln!("{}", e);
            std::process::exit(1);
        });

    println!("Loaded and verified config:\n{}", cfg)
}
