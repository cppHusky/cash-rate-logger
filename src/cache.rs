use std::path::PathBuf;

use anyhow::Context;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct CacheEntry {
    pub timestamp: u64,
    pub base: String,
    pub rates: std::collections::HashMap<String, f64>,
}

fn cache_dir() -> PathBuf {
    let home = std::env::var("HOME").expect("HOME environment variable not set");
    PathBuf::from(home).join(".cache").join("cash-rate-logger")
}

fn ensure_cache_dir() -> anyhow::Result<PathBuf> {
    let dir = cache_dir();
    std::fs::create_dir_all(&dir).context("failed to create cache directory")?;
    Ok(dir)
}

pub fn get_latest() -> anyhow::Result<Option<CacheEntry>> {
    let dir = cache_dir();
    if !dir.exists() {
        return Ok(None);
    }

    let mut latest_ts: Option<u64> = None;
    for entry in std::fs::read_dir(&dir).context("failed to read cache directory")? {
        let entry = entry?;
        let fname = entry.file_name();
        let name = fname.to_string_lossy();
        if let Some(stem) = name.strip_suffix(".json") {
            if let Ok(ts) = stem.parse::<u64>() {
                if let Some(current) = latest_ts {
                    if ts > current {
                        latest_ts = Some(ts);
                    }
                } else {
                    latest_ts = Some(ts);
                }
            }
        }
    }

    match latest_ts {
        Some(ts) => {
            let path = dir.join(format!("{}.json", ts));
            let content = std::fs::read_to_string(&path)
                .with_context(|| format!("failed to read cache file: {}", path.display()))?;
            let entry: CacheEntry = serde_json::from_str(&content)
                .with_context(|| format!("failed to parse cache file: {}", path.display()))?;
            Ok(Some(entry))
        }
        None => Ok(None),
    }
}

pub fn write(timestamp: u64, entry: &CacheEntry) -> anyhow::Result<()> {
    let dir = ensure_cache_dir()?;
    let path = dir.join(format!("{}.json", timestamp));
    let json = serde_json::to_string_pretty(entry).context("failed to serialize cache entry")?;
    std::fs::write(&path, json)
        .with_context(|| format!("failed to write cache file: {}", path.display()))?;
    Ok(())
}
