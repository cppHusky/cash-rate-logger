use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct ConfigFile {
    pub base: Option<String>,
    pub output: Option<Vec<String>>,
}

#[derive(Debug)]
pub struct Config {
    pub base: String,
    pub output: Option<Vec<String>>,
}

impl Config {
    pub fn load(cli_base: Option<String>, cli_output: Option<Vec<String>>, config_path: Option<&str>) -> anyhow::Result<Self> {
        let mut base = cli_base;
        let mut output = cli_output;

        if let Some(path) = config_path {
            let content = std::fs::read_to_string(path)?;
            let file: ConfigFile = toml::from_str(&content)?;
            if base.is_none() {
                base = file.base;
            }
            if output.is_none() {
                output = file.output;
            }
        }

        Ok(Config {
            base: base.unwrap_or_else(|| "USD".to_string()),
            output,
        })
    }
}
