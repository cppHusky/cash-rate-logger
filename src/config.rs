use clap::ValueEnum;
use serde::Deserialize;

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq, ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum HistoryDirection {
    High,
    Low,
}

#[derive(Debug, Clone, Copy)]
pub struct HistoryConfig {
    pub days: u64,
    pub percent: f64,
    pub direction: HistoryDirection,
}

#[derive(Debug, Deserialize)]
pub struct ConfigFile {
    pub base: Option<String>,
    pub output: Option<Vec<String>>,
    pub history_days: Option<u64>,
    pub history_percent: Option<f64>,
    pub history_direction: Option<HistoryDirection>,
}

#[derive(Debug)]
pub struct Config {
    pub base: String,
    pub output: Option<Vec<String>>,
    pub history: Option<HistoryConfig>,
}

impl Config {
    pub fn load(
        cli_base: Option<String>,
        cli_output: Option<Vec<String>>,
        config_path: Option<&str>,
        cli_history_days: Option<u64>,
        cli_history_percent: Option<f64>,
        cli_history_direction: Option<HistoryDirection>,
    ) -> anyhow::Result<Self> {
        let mut base = cli_base;
        let mut output = cli_output;
        let mut history_days = cli_history_days;
        let mut history_percent = cli_history_percent;
        let mut history_direction = cli_history_direction;

        if let Some(path) = config_path {
            let content = std::fs::read_to_string(path)?;
            let file: ConfigFile = toml::from_str(&content)?;
            if base.is_none() {
                base = file.base;
            }
            if output.is_none() {
                output = file.output;
            }
            if history_days.is_none() {
                history_days = file.history_days;
            }
            if history_percent.is_none() {
                history_percent = file.history_percent;
            }
            if history_direction.is_none() {
                history_direction = file.history_direction;
            }
        }

        let history = match (history_days, history_percent, history_direction) {
            (None, None, None) => None,
            (Some(days), Some(percent), Some(direction)) => {
                if days == 0 {
                    anyhow::bail!("history days must be greater than 0");
                }
                if !percent.is_finite() || !(0.0..=100.0).contains(&percent) || percent == 0.0 {
                    anyhow::bail!("history percent must be greater than 0 and at most 100");
                }
                Some(HistoryConfig {
                    days,
                    percent,
                    direction,
                })
            }
            _ => anyhow::bail!(
                "history days, history percent, and history direction must be specified together"
            ),
        };

        Ok(Config {
            base: base.unwrap_or_else(|| "USD".to_string()),
            output,
            history,
        })
    }
}
