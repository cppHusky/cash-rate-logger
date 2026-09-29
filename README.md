# A cached cash rate logger

`cash-rate-logger` fetches exchange rates from Open Exchange Rates, stores
complete responses in `~/.cache/cash-rate-logger/`, and evaluates them using a
custom display currency.

## History monitoring

The current rate can be compared with records from the previous rolling number
of days. Choose one direction to watch:

```sh
cash-rate-logger get \
  --base USD \
  --output EUR \
  --history-direction high \
  --history-percent 80 \
  --history-days 30
```

`high` matches when the current value is higher than at least the configured
percentage of historical records. `low` matches when it is lower than at least
that percentage. The current record is not included in its own comparison.

The same settings can be placed in a TOML config file:

```toml
base = "USD"
output = ["EUR"]
history_direction = "low"
history_percent = 80
history_days = 30
```

All three history settings must be provided together. CLI values override the
config file.

## Waybar

Waybar can display multiple output currencies. Each currency receives its own
history percentage and arrow in the tooltip. The Waybar class becomes
`history-alert` when any displayed currency meets the configured condition:

```json
"custom/cash-rate": {
    "exec": "cash-rate-logger get --waybar --output EUR,JPY --history-direction high --history-percent 80 --history-days 30",
    "return-type": "json",
    "interval": 300
}
```

When any displayed currency matches, the JSON output has the class
`history-alert`; otherwise it has the class `normal`. Color it in the Waybar
stylesheet:

```css
#custom-cash-rate.history-alert {
    color: #98c379;
}
```

## Cron notifications

Use `--notification` to produce one combined notification message for every
selected currency that matches the history condition. The output is empty when
no currency matches, including when only one currency was selected:

```sh
message=$(cash-rate-logger get \
    --base CNY \
    --output JPY,EUR \
    --history-direction low \
    --history-percent 95 \
    --history-days 7 \
    --notification)

if [ -n "$message" ]; then
    notify-send --app-name="cash-rate-logger" "Cash Rate Notice" "$message"
fi
```

Notification mode requires an output filter and all history settings. It does
not call `notify-send` itself, so it can also be used by other notification
systems.
