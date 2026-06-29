# polymarket_1x2

A read-only-by-default Rust collector for Polymarket football 1X2 market quotes and scores plus
OddsPortal bookmaker odds and scores. One shared team pair drives independent discovery at both
providers.

## Build and run

```bash
cargo build
cargo run >observations.jsonl 2>diagnostics.log
```

The binary reads `config.yaml` from the working directory. Keep the committed safety gate
disabled:

```yaml
proxy: "http://127.0.0.1:7890"
gamma_host: https://gamma-api.polymarket.com
host: https://clob.polymarket.com
chain_id: 137

match:
  home_team: South Africa
  away_team: Canada

polymarket:
  enabled: true
  log_path: logs/polymarket_quotes.log

oddsportal:
  enabled: true
  tournament_url: https://www.oddsportal.com/football/world/world-championship-2026/
  log_path: logs/oddsportal_odds.log
  poll_interval_seconds: 1

trade:
  enabled: false
  trader_mode: real
  account_mode: real
  market_mode: real
```

Replace the proxy example through a local, untracked configuration when required. Do not put
credentials or a private proxy URL in source control. `trade.enabled` remains `false`; the three
mode values alone never enable order placement.

## Output

Stdout contains only one JSON object per line. The four observation types have these shapes
(values are illustrative):

```json
{"provider":"polymarket","type":"polymarket_odds","received_at":"2026-06-28T12:00:00.000Z","source_updated_at":null,"event_slug":"fifwc-rsa-can-2026-06-28","home_team":"South Africa","away_team":"Canada","result":"home","market_slug":"rsa-win","asset_id":"11","bid_price":"0.16","bid_size":"100","ask_price":"0.17","ask_size":"80","source":"book"}
{"provider":"polymarket","type":"polymarket_score","received_at":"2026-06-28T12:32:15.000Z","source_updated_at":"2026-06-28T12:32:14Z","event_slug":"fifwc-rsa-can-2026-06-28","home_team":"South Africa","away_team":"Canada","score":"1-0","status":"InProgress","period":"1H","elapsed":"32:15","live":true,"ended":false}
{"provider":"oddsportal","type":"oddsportal_odds","received_at":"2026-06-28T12:00:00.000Z","source_updated_at":null,"event_id":"EZmXxG15","event_name":"South Africa - Canada","home_team":"South Africa","away_team":"Canada","bookmakers":[{"bookmaker_id":"16","bookmaker_name":"bet365","outcomes":{"1":"5.50","2":"1.62","X":"3.80"}}]}
{"provider":"oddsportal","type":"oddsportal_score","received_at":"2026-06-28T12:00:00.000Z","source_updated_at":null,"event_id":"EZmXxG15","event_name":"South Africa - Canada","home_team":"South Africa","away_team":"Canada","available":false,"score":null,"status":null,"period":null,"elapsed":null}
```

Polymarket uses separate market-data and public sports-score WebSockets. At every non-overlapping
OddsPortal polling tick, one odds operation and one score operation start concurrently. The score
operation makes zero HTTP calls if discovery found no score URL, otherwise one. The odds
operation makes one primary call and may make exactly one fallback call if the primary fails or
is empty. A cycle therefore makes one to three HTTP calls, normally two when a score URL exists
and primary odds succeeds. The committed interval is one second, but OddsPortal advertises an
approximately 15-second source refresh; faster requests cannot make the upstream data refresh
and may be rate-limited.

Provider-prefixed discovery, lifecycle, retry, reconnect, and failure diagnostics go to stderr.
The configured `polymarket.log_path` and `oddsportal.log_path` retain the legacy detailed quote
records; score observations are available on stdout only.

For release deployment and process management, see [DEPLOYMENT.md](DEPLOYMENT.md).
