use crate::manifest::Manifest;
use tokio::fs as tokio_fs;
use tokio::io::AsyncWriteExt;

pub async fn fetch_manifest(endpoint: &str) -> Result<Manifest, Box<dyn std::error::Error>> {
    let url = endpoint
        .replace("github.com", "raw.githubusercontent.com")
        .replace("/blob/", "/");

    let response = reqwest::get(&url).await?.error_for_status()?;
    let content = response.text().await?;
    let manifest: Manifest = toml::from_str(&content)?;
    Ok(manifest)
}

pub async fn download_firmware(
        manifest: &Manifest,
        staging_dir: &str,
    ) -> Result<String, Box<dyn std::error::Error>> {
    println!("Downloading firmware from {}", manifest.download_url);

    let response = reqwest::get(&manifest.download_url).await?.error_for_status()?;
    
    // Extract filename from URL
    let artifact_name = manifest.download_url.split('/').last()
        .ok_or("Could not extract filename from download URL")?;

    tokio_fs::create_dir_all(staging_dir).await?;
    
    let dest_path_str = format!("{}/{}", staging_dir, artifact_name);
    let mut file = tokio_fs::File::create(&dest_path_str).await?;
    
    let content = response.bytes().await?;
    file.write_all(&content).await?;

    Ok(dest_path_str)
}
