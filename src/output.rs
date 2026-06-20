use serde::Serialize;

use crate::cache::CacheEntry;

#[derive(Serialize)]
struct WaybarOutput {
    text: String,
    tooltip: String,
}

fn compute_rates(entry: &CacheEntry, base: &str, output_filter: &Option<Vec<String>>) -> anyhow::Result<Vec<(String, f64)>> {
    let base_rate = entry.rates.get(base).cloned();
    let base_rate = match base_rate {
        Some(r) if r != 0.0 => r,
        Some(_) => anyhow::bail!("base currency '{}' has rate 0.0", base),
        None => anyhow::bail!("base currency '{}' not found in rates", base),
    };

    let mut results: Vec<(String, f64)> = entry
        .rates
        .iter()
        .filter(|(code, _)| {
            if let Some(ref filter) = output_filter {
                filter.contains(code)
            } else {
                true
            }
        })
        .map(|(code, rate)| (code.clone(), 100.0 * rate / base_rate))
        .collect();

    results.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(results)
}

pub fn format_terminal(entry: &CacheEntry, base: &str, output_filter: &Option<Vec<String>>) -> anyhow::Result<String> {
    let rates = compute_rates(entry, base, output_filter)?;
    let mut out = format!("100 {}:\n", base);
    for (code, value) in &rates {
        out.push_str(&format!("  {}: {:.2}\n", code, value));
    }
    Ok(out)
}

pub fn format_waybar(entry: &CacheEntry, base: &str, output_filter: &Option<Vec<String>>) -> anyhow::Result<String> {
    let rates = compute_rates(entry, base, output_filter)?;

    let first = rates.first().map(|(c, v)| format!("{}: {:.2}", c, v)).unwrap_or_default();

    let header = format!("100 {}:", base);
    let tooltip = std::iter::once(header)
        .chain(rates.iter().map(|(c, v)| format!("{}: {:.2}", c, v)))
        .collect::<Vec<_>>()
        .join("\n");

    let output = WaybarOutput { text: first, tooltip };
    Ok(serde_json::to_string(&output).expect("serialization should not fail"))
}
