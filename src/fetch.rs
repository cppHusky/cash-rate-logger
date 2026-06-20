use std::collections::HashMap;

use anyhow::{Context, bail};
use serde::Deserialize;

use crate::cache::{CacheEntry, self};

#[derive(Debug, Deserialize)]
struct ApiResponse {
    #[serde(default)]
    timestamp: Option<u64>,
    base: String,
    rates: HashMap<String, f64>,
    #[serde(default)]
    error: Option<bool>,
    #[serde(default)]
    description: Option<String>,
}

pub fn fetch_rates(app_id: &str) -> anyhow::Result<CacheEntry> {
    let url = format!("https://openexchangerates.org/api/latest.json?app_id={}", app_id);

    let response: ApiResponse =
        ureq::get(&url).call().context("API request failed")?.into_json().context("failed to parse API response")?;

    if response.error.unwrap_or(false) {
        bail!(
            "API error: {}",
            response.description.as_deref().unwrap_or("unknown error")
        );
    }

    let timestamp = response.timestamp.unwrap_or_else(|| {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
    });

    Ok(CacheEntry { timestamp, base: response.base, rates: response.rates })
}

pub fn fetch_and_cache(app_id: &str) -> anyhow::Result<()> {
    let entry = fetch_rates(app_id)?;
    let ts = entry.timestamp;
    cache::write(ts, &entry)?;
    eprintln!("Cached rates at timestamp {}", ts);
    Ok(())
}
