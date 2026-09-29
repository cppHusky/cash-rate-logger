mod cache;
mod config;
mod fetch;
mod output;

use anyhow::Context;
use clap::{Parser, Subcommand};

use crate::config::HistoryDirection;

#[derive(Parser, Debug)]
#[command(name = "cash-rate-logger", about = "cash rate with cache")]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,

    #[arg(
        short = 'b',
        long = "base",
        help = "the base currency you want, Accepts ISO 4217 codes"
    )]
    base: Option<String>,

    #[arg(
        short = 'o',
        long = "output",
        value_delimiter = ',',
        help = "filter the currency codes you want to output"
    )]
    output: Option<Vec<String>>,

    #[arg(short = 'c', long = "config", help = "the config file you want to use")]
    config: Option<String>,

    #[arg(long = "waybar", help = "output as waybar tooltip JSON")]
    waybar: bool,

    #[arg(long = "notification", help = "output a system notification message")]
    notification: bool,

    #[arg(
        long = "history-direction",
        visible_alias = "history",
        value_enum,
        help = "watch a high or low historical rank"
    )]
    history_direction: Option<HistoryDirection>,

    #[arg(
        long = "history-percent",
        visible_alias = "history-percentile",
        help = "percent of history the current value must exceed in the selected direction"
    )]
    history_percent: Option<f64>,

    #[arg(long = "history-days", help = "number of previous days to compare")]
    history_days: Option<u64>,
}

#[derive(Subcommand, Debug)]
enum Command {
    #[command(about = "get the output from cache. do not update cash rate")]
    Get {
        #[arg(short = 'b', long = "base")]
        base: Option<String>,

        #[arg(short = 'o', long = "output", value_delimiter = ',')]
        output: Option<Vec<String>>,

        #[arg(short = 'c', long = "config")]
        config: Option<String>,

        #[arg(long = "waybar")]
        waybar: bool,

        #[arg(long = "notification")]
        notification: bool,

        #[arg(long = "history-direction", visible_alias = "history", value_enum)]
        history_direction: Option<HistoryDirection>,

        #[arg(long = "history-percent", visible_alias = "history-percentile")]
        history_percent: Option<f64>,

        #[arg(long = "history-days")]
        history_days: Option<u64>,
    },

    #[command(about = "fetch current cash rate from API and store into cache")]
    Fetch,
}

fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    let cli = Cli::parse();

    match &cli.command {
        Some(Command::Fetch) => {
            let app_id = std::env::var("OPEN_EXCHANGE_RATE_APP_ID")
                .context("OPEN_EXCHANGE_RATE_APP_ID not set in environment or .env file")?;
            fetch::fetch_and_cache(&app_id)?;
        }
        Some(Command::Get {
            base,
            output,
            config,
            waybar,
            history_direction,
            history_percent,
            history_days,
            notification,
        }) => {
            let cfg = config::Config::load(
                first_some(base.clone(), cli.base.clone()),
                first_some(output.clone(), cli.output.clone()),
                first_some(config.as_deref(), cli.config.as_deref()),
                first_some(*history_days, cli.history_days),
                first_some(*history_percent, cli.history_percent),
                first_some(*history_direction, cli.history_direction),
            )?;
            run_get(
                &cfg,
                *waybar || cli.waybar,
                *notification || cli.notification,
            )?;
        }
        None => {
            let cfg = config::Config::load(
                cli.base.clone(),
                cli.output.clone(),
                cli.config.as_deref(),
                cli.history_days,
                cli.history_percent,
                cli.history_direction,
            )?;
            run_get(&cfg, cli.waybar, cli.notification)?;
        }
    }

    Ok(())
}

fn first_some<T>(a: Option<T>, b: Option<T>) -> Option<T> {
    a.or(b)
}

fn run_get(cfg: &config::Config, waybar: bool, notification: bool) -> anyhow::Result<()> {
    let history = cache::get_history()?;
    let entry = history
        .last()
        .cloned()
        .context("no cached data found. run 'fetch' first")?;

    if waybar && notification {
        anyhow::bail!("--waybar and --notification cannot be used together");
    } else if notification {
        print!(
            "{}",
            output::format_notification(
                &entry,
                &history,
                &cfg.base,
                &cfg.output,
                cfg.history.as_ref(),
            )?
        );
    } else if waybar {
        print!(
            "{}",
            output::format_waybar(
                &entry,
                &history,
                &cfg.base,
                &cfg.output,
                cfg.history.as_ref(),
            )?
        );
    } else {
        print!(
            "{}",
            output::format_terminal(
                &entry,
                &history,
                &cfg.base,
                &cfg.output,
                cfg.history.as_ref(),
            )?
        );
    }

    Ok(())
}
