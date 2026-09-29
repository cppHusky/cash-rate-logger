use serde::Serialize;

use crate::cache::CacheEntry;
use crate::config::{HistoryConfig, HistoryDirection};

const SECONDS_PER_DAY: u64 = 24 * 60 * 60;

#[derive(Serialize)]
struct WaybarOutput {
    text: String,
    tooltip: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    class: Option<&'static str>,
}

#[derive(Debug, Clone, Copy)]
struct HistoryComparison {
    percent: f64,
    matched: bool,
}

fn base_rate(entry: &CacheEntry, base: &str) -> anyhow::Result<f64> {
    match entry.rates.get(base).copied() {
        Some(rate) if rate != 0.0 => Ok(rate),
        Some(_) => anyhow::bail!("base currency '{}' has rate 0.0", base),
        None => anyhow::bail!("base currency '{}' not found in rates", base),
    }
}

fn compute_rates(
    entry: &CacheEntry,
    base: &str,
    output_filter: &Option<Vec<String>>,
) -> anyhow::Result<Vec<(String, f64)>> {
    let base_rate = base_rate(entry, base)?;

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

fn converted_rate(entry: &CacheEntry, base: &str, currency: &str) -> Option<f64> {
    let base_rate = entry.rates.get(base).copied()?;
    let currency_rate = entry.rates.get(currency).copied()?;

    if !base_rate.is_finite() || base_rate == 0.0 || !currency_rate.is_finite() {
        return None;
    }

    let converted = 100.0 * currency_rate / base_rate;
    converted.is_finite().then_some(converted)
}

fn compare_history(
    current: &CacheEntry,
    history: &[CacheEntry],
    base: &str,
    currency: &str,
    config: &HistoryConfig,
) -> Option<HistoryComparison> {
    let current_value = converted_rate(current, base, currency)?;
    let max_age = config.days.saturating_mul(SECONDS_PER_DAY);
    let historical_values: Vec<f64> = history
        .iter()
        .filter_map(|entry| {
            if entry.timestamp >= current.timestamp || current.timestamp - entry.timestamp > max_age
            {
                return None;
            }
            converted_rate(entry, base, currency)
        })
        .collect();

    if historical_values.is_empty() {
        return None;
    }

    let matching_records = match config.direction {
        HistoryDirection::High => historical_values
            .iter()
            .filter(|value| **value < current_value)
            .count(),
        HistoryDirection::Low => historical_values
            .iter()
            .filter(|value| **value > current_value)
            .count(),
    };
    let percent = 100.0 * matching_records as f64 / historical_values.len() as f64;

    Some(HistoryComparison {
        percent,
        matched: percent >= config.percent,
    })
}

fn direction_arrow(direction: HistoryDirection) -> &'static str {
    match direction {
        HistoryDirection::High => "↑ than",
        HistoryDirection::Low => "↓ than",
    }
}

fn history_suffix(config: &HistoryConfig, comparison: Option<HistoryComparison>) -> String {
    match comparison {
        Some(comparison) => {
            format!(
                " {} {:.1}%",
                direction_arrow(config.direction),
                comparison.percent
            )
        }
        None => format!(" {} n/a", direction_arrow(config.direction)),
    }
}

pub fn format_terminal(
    entry: &CacheEntry,
    history: &[CacheEntry],
    base: &str,
    output_filter: &Option<Vec<String>>,
    history_config: Option<&HistoryConfig>,
) -> anyhow::Result<String> {
    let rates = compute_rates(entry, base, output_filter)?;
    let mut out = format!("100 {}:\n", base);
    for (code, value) in &rates {
        let history = history_config
            .map(|config| {
                history_suffix(config, compare_history(entry, history, base, code, config))
            })
            .unwrap_or_default();
        out.push_str(&format!(
            "  {}: {:.3} ({:.3}){}\n",
            code,
            value,
            10000. / value,
            history
        ));
    }
    Ok(out)
}

pub fn format_waybar(
    entry: &CacheEntry,
    history: &[CacheEntry],
    base: &str,
    output_filter: &Option<Vec<String>>,
    history_config: Option<&HistoryConfig>,
) -> anyhow::Result<String> {
    let mut rates = compute_rates(entry, base, output_filter)?;
    if let Some(filter) = output_filter {
        let ordered_rates: Vec<(String, f64)> = filter
            .iter()
            .filter_map(|currency| rates.iter().find(|(code, _)| code == currency).cloned())
            .fold(Vec::new(), |mut ordered, rate| {
                if !ordered.iter().any(|(code, _)| code == &rate.0) {
                    ordered.push(rate);
                }
                ordered
            });
        rates = ordered_rates;
    }

    let history_comparisons = history_config.map(|config| {
        rates
            .iter()
            .map(|(currency, _)| {
                (
                    currency.clone(),
                    compare_history(entry, history, base, currency, config),
                )
            })
            .collect::<Vec<_>>()
    });
    let comparison_for = |currency: &str| {
        history_comparisons.as_ref().and_then(|comparisons| {
            comparisons
                .iter()
                .find(|(code, _)| code == currency)
                .and_then(|(_, comparison)| *comparison)
        })
    };

    let first_rate = rates.first();
    let first = first_rate
        .map(|(c, v)| {
            let history = history_config
                .map(|config| history_suffix(config, comparison_for(c)))
                .unwrap_or_default();
            format!("{}: {:.3} ({:.3}){}", c, v, 10000. / v, history)
        })
        .unwrap_or_default();

    let header = format!("100 {}:", base);
    let mut tooltip_lines = vec![header];
    tooltip_lines.extend(rates.iter().map(|(c, v)| {
        let history = history_config
            .map(|config| history_suffix(config, comparison_for(c)))
            .unwrap_or_default();
        format!("{}: {:.3} ({:.3}){}", c, v, 10000. / v, history)
    }));

    let class = if let Some(comparisons) = &history_comparisons {
        Some(
            if comparisons
                .iter()
                .any(|(_, comparison)| comparison.map(|value| value.matched).unwrap_or(false))
            {
                "history-alert"
            } else {
                "normal"
            },
        )
    } else {
        None
    };

    let output = WaybarOutput {
        text: first,
        tooltip: tooltip_lines.join("\n"),
        class,
    };
    Ok(serde_json::to_string(&output).expect("serialization should not fail"))
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    fn entry(timestamp: u64, eur: f64) -> CacheEntry {
        CacheEntry {
            timestamp,
            base: "USD".to_string(),
            rates: HashMap::from([("USD".to_string(), 1.0), ("EUR".to_string(), eur)]),
        }
    }

    fn config(direction: HistoryDirection) -> HistoryConfig {
        HistoryConfig {
            days: 30,
            percent: 80.0,
            direction,
        }
    }

    #[test]
    fn high_direction_compares_against_lower_history_values() {
        let current = entry(4, 4.0);
        let history = vec![entry(1, 1.0), entry(2, 2.0), entry(3, 3.0), current.clone()];

        let result = compare_history(
            &current,
            &history,
            "USD",
            "EUR",
            &config(HistoryDirection::High),
        )
        .expect("history should be available");

        assert_eq!(result.percent, 100.0);
        assert!(result.matched);
    }

    #[test]
    fn low_direction_compares_against_higher_history_values() {
        let current = entry(4, 1.0);
        let history = vec![entry(1, 2.0), entry(2, 3.0), entry(3, 4.0), current.clone()];

        let result = compare_history(
            &current,
            &history,
            "USD",
            "EUR",
            &config(HistoryDirection::Low),
        )
        .expect("history should be available");

        assert_eq!(result.percent, 100.0);
        assert!(result.matched);
    }

    #[test]
    fn records_outside_the_requested_window_are_ignored() {
        let current = entry(10 * SECONDS_PER_DAY, 4.0);
        let history = vec![
            entry(SECONDS_PER_DAY, 1.0),
            entry(8 * SECONDS_PER_DAY, 5.0),
            current.clone(),
        ];
        let config = HistoryConfig {
            days: 3,
            percent: 80.0,
            direction: HistoryDirection::Low,
        };

        let result = compare_history(&current, &history, "USD", "EUR", &config)
            .expect("history should be available");

        assert_eq!(result.percent, 100.0);
        assert!(result.matched);
    }

    #[test]
    fn waybar_marks_a_matching_history_record() {
        let current = entry(4, 4.0);
        let history = vec![entry(1, 1.0), entry(2, 2.0), entry(3, 3.0), current.clone()];
        let output = format_waybar(
            &current,
            &history,
            "USD",
            &Some(vec!["EUR".to_string()]),
            Some(&config(HistoryDirection::High)),
        )
        .expect("Waybar output should be valid");
        let json: serde_json::Value = serde_json::from_str(&output).expect("valid JSON");

        assert_eq!(json["class"], "history-alert");
        assert!(json["text"].as_str().unwrap().contains("↑ 100.0%"));
        assert!(!json["tooltip"].as_str().unwrap().contains("history:"));
    }

    #[test]
    fn waybar_displays_rank_for_every_currency_and_alerts_on_any_match() {
        let current = entry_with_currencies(4, 0.5, 4.0);
        let history = vec![
            entry_with_currencies(1, 1.0, 1.0),
            entry_with_currencies(2, 2.0, 2.0),
            entry_with_currencies(3, 3.0, 3.0),
            current.clone(),
        ];
        let output = format_waybar(
            &current,
            &history,
            "USD",
            &Some(vec!["JPY".to_string(), "EUR".to_string()]),
            Some(&config(HistoryDirection::Low)),
        )
        .expect("Waybar output should be valid");
        let json: serde_json::Value = serde_json::from_str(&output).expect("valid JSON");
        let text = json["text"].as_str().unwrap();
        let tooltip = json["tooltip"].as_str().unwrap();

        assert!(text.starts_with("JPY:"));
        assert!(text.contains("↓ 0.0%"));
        assert!(tooltip.contains("JPY: 400.000 (25.000) ↓ 0.0%"));
        assert!(tooltip.contains("EUR: 50.000 (200.000) ↓ 100.0%"));
        assert_eq!(json["class"], "history-alert");
    }

    fn entry_with_currencies(timestamp: u64, eur: f64, jpy: f64) -> CacheEntry {
        CacheEntry {
            timestamp,
            base: "USD".to_string(),
            rates: HashMap::from([
                ("USD".to_string(), 1.0),
                ("EUR".to_string(), eur),
                ("JPY".to_string(), jpy),
            ]),
        }
    }
}
