# mawan

**Ma Wan / Park Island transport in one binary — CLI and MCP server.**

`mawan` pulls the live feeds behind the Park Island ETA app and Hong Kong's
open bus data, pairs them with the operators' published timetables, ferries,
fares and notices, and exposes everything over a CLI and MCP.

- **Live bus ETA:** PITCL NR330, NR332, NR334, NR338; KMB 230R
- **Bus timetables:** NR330, NR332, NR334, NR338, NR338S, NR331, NR331S, 230R
- **Fares:** published adult, child and elderly fares where the operator lists them
- **Ferries:** Park Island ↔ Central Pier No. 2 and Park Island ↔ Tsuen Wan
- **Weather:** current HKO conditions, active warnings and the local forecast
- **Traffic:** real-time TD special traffic news, filtered to the Ma Wan corridor
- **PITCL news:** latest operator notices in English or Traditional Chinese
- **PITCL guidance:** hotline, typhoon/high-wind plan and passenger policies
- **Stops and directions** for every bus route
- **MCP server** over stdio and streamable HTTP
- **Agent skill** (`SKILL.md`) plus a binary bootstrap script

No API key. Times are Hong Kong time (UTC+8).

## Bus routes

| Route | Operator | Service | Live data |
| --- | --- | --- | --- |
| NR330 | PITCL | Park Island ↔ MTR Tsing Yi Station | yes |
| NR332 | PITCL | Park Island ↔ MTR Kwai Fong Station | yes |
| NR334 | PITCL | Park Island ↺ Hong Kong International Airport | yes |
| NR338 | PITCL | Park Island ↔ Central (Pier 2), overnight | yes |
| NR338S | PITCL | Ma Wan ↔ Central via Hong Kong Station | timetable |
| NR331 | Sun Bus | Ma Wan (Pak Yan Road) ↔ Tsuen Wan Station | timetable |
| NR331S | Sun Bus | Ma Wan (Pak Yan Road) ↔ Tsuen Wan West | timetable |
| 230R | KMB | Kowloon Station ↔ Ma Wan (Pak Yan Road) | yes |

230R is the exception to the PITCL pattern: it is operated by KMB, not Park
Island Transport, and its live ETAs come from KMB's open data at
`data.etabus.gov.hk`. The same commands work: `mawan eta 230R`, `mawan stops
230R`, `mawan schedule 230R`.

### Fares

| Route | Adult visitor | Adult resident | Notes |
| --- | --- | --- | --- |
| NR330 | HK$13.80 | HK$8.50 | registered Octopus for resident fare |
| NR332 | HK$14.50 | HK$9.80 | registered Octopus for resident fare |
| NR334 | HK$32.50 | — | no resident fare; $2 concession not applicable |
| NR338 | HK$32.00 | HK$24.50 | overnight |
| NR338S | HK$39.00 | HK$24.10 | child $19.50/$12.10, elderly $12.10/$12.10; resident offer $22.00/$11.00 until 31 Dec 2026 |
| 230R | HK$15.50 | — | KMB fare |

Use `mawan route <code>` for the full fare object, including child and
elderly fares. Sun Bus does not publish NR331/NR331S fares in the sources used
here, so those are omitted rather than guessed.

## Ferries

| Route | Other pier | Weekday sailings | Weekend/holiday sailings |
| --- | --- | --- | --- |
| `central` | Central Pier No. 2 | 19 each way | 7 each way |
| `tsuen-wan` | Tsuen Wan Pier | 3 each way | 3 each way |

Ferry times are pier gate-closing times. Fares: Central $36.00 adult visitor /
$24.10 resident; Tsuen Wan $12.00 / $10.20. Child and elderly fares are
roughly half and are returned in full by `mawan ferry routes --json`.

## Weather impact

`mawan weather` includes a `pitcl_impact` object with `level` (`normal`,
`watch`, `disruption`, `severe`) and separate bus/ferry guidance. It does not
treat a T8 as an automatic bus cancellation:

- **T8:** ferries suspend about 2 hours after the signal is hoisted; Tsuen Wan
  ferry may be strengthened until crowds clear. Buses may continue until
  Stage 2 high-wind management on Tsing Ma Bridge (>75 km/h) suspends them.
- **T3:** ferries may be limited or suspended; buses normally continue.
- **T1:** ferries may be delayed; buses normally continue.
- **Rainstorm:** expect delays; PITCL's published plan has no automatic
  suspension.

The impact text is English-only; the warnings and forecast follow `--lang tc`.

## Install

```bash
git clone <this repository>
cd hongkong-pitcl-skill
cargo install --path . --locked
# or: cargo build --release --locked   # target/release/mawan
```

Requires Rust 1.89 or newer.

## CLI

```bash
mawan routes                       # every Ma Wan route and its data source
mawan route NR330                  # both directions and all stops
mawan stops NR330 -d 1             # stops, direction 1
mawan stops 230R -d 2

mawan eta NR330                    # live arrivals at every stop
mawan eta NR330 -d 2 -s "Tsing Yi" # filter by stop name
mawan eta NR331                    # explains that no live feed exists

mawan schedule NR330 --day weekday   # exact weekday times
mawan schedule NR330 --day saturday  # frequency windows
mawan schedule NR338 --day today
mawan schedule 230R --day sunday
mawan schedule NR338S --day 2026-12-25

mawan news                          # latest PITCL notices, newest first
mawan news --id 61                  # read one notice in full
mawan news --lang tc --limit 5

mawan ferry routes                  # ferry routes, piers and fares
mawan ferry schedule central --day weekday
mawan ferry schedule tsuen-wan --day sunday --json

mawan weather                        # HKO conditions, warnings and forecast
mawan weather --lang tc
mawan traffic                        # TD news affecting Ma Wan
mawan traffic --all --limit 10       # every territory-wide message

mawan guide contact                  # hotline, email, resident fare forms
mawan guide weather                  # high-wind and typhoon arrangements
mawan guide policies                 # ferry luggage, pets and on-board rules
```

Global flags: `--json`, `--lang en|tc`, `--cache-ttl 30`.

Direction `1` is the operator's outbound direction and `2` is inbound. For
230R that means `1` is Kowloon Station → Ma Wan.

### Examples

```console
$ mawan eta NR332 -d 1 -s "Kwai Fong"
SEQ  STOP                 ETA    IN      SOURCE
---  -------------------  -----  ------  --------
4    MTR Kwai Fong Station  07:42  8 min  schedule
```

```console
$ mawan schedule NR330 --day saturday
NR330  dir 1  Park Island → MTR Tsing Yi Station  [saturday / official-frequency]
First 06:00  ·  Last 00:30

06:00–06:30  every 15 min
06:30–07:30  every 10 min
...
```

## MCP server

```bash
mawan mcp                          # stdio
mawan mcp --transport http --port 8788
```

Client configuration for stdio:

```json
{
  "mcpServers": {
    "mawan": {
      "command": "mawan",
      "args": ["mcp"]
    }
  }
}
```

HTTP transport serves `POST /mcp` and `GET /health` on `127.0.0.1:8788`.

Tools:

| Tool | Purpose |
| --- | --- |
| `list_ma_wan_routes` | route, operator, termini, live availability |
| `get_ma_wan_route` | one route, both directions, stops, fares |
| `get_ma_wan_stops` | ordered stops for a direction |
| `get_ma_wan_arrivals` | live ETAs, optional stop filter |
| `get_ma_wan_schedule` | exact times or frequency windows by day type |
| `get_ma_wan_news` | latest PITCL notices; pass an `id` for full text |
| `list_ma_wan_ferries` | ferry routes, piers and fares |
| `get_ma_wan_ferry_schedule` | ferry sailings by day type |
| `get_ma_wan_weather` | HKO weather, warnings, forecast and PITCL impact |
| `get_ma_wan_traffic` | TD special traffic news, Ma Wan-related by default |
| `get_pitcl_guide` | contact, adverse-weather plan and passenger policies |

## Agent skill

```bash
npx skills add <owner>/hongkong-pitcl-skill
```

Then run `bash scripts/ensure-binary.sh` once. It checks `$MAWAN_BIN`, `mawan`
on `PATH`, and finally builds from the checkout, printing the absolute binary
path to use.

## Official links and feeds

- Bus services: <https://www.pitcl.com.hk/en/bus_service.php>
- Ferries: <https://www.pitcl.com.hk/en/ferry_service.php>
- Notices: <https://www.pitcl.com.hk/en/news.php>
- Weather: <https://data.weather.gov.hk/weatherAPI/opendata/weather.php>
- Traffic: <https://resource.data.one.gov.hk/td/en/specialtrafficnews.xml>

These replace scraping the older `traffic.td.gov.hk`, RTHK and HKO pages:
HKO's open-data API and TD's special-traffic-news XML are machine-readable and
update in real time.

## Data sources

- **PITCL NR330/332/334/338** — the official Park Island ETA app at
  `eta.pitcl.com.hk` is a Socif deployment. Its public JSON API at
  `360-api.socif.co` exposes route metadata, stops, per-stop ETAs and
  weekday timetables. Route IDs: NR330=249, NR332=250, NR334=252, NR338=253.
  The weekend/seasonal headways come from the same metadata. Signed
  `timestamp` values are converted to UTC+8.
- **230R** — KMB open data (`data.etabus.gov.hk`) for route stops and live
  ETAs; the frequency table is the published KMB/TD timetable.
- **NR338S** — PITCL leaflet *Park Island – Central (Via Hong Kong Stn.)*,
  Ver. 20250901, effective 1 September 2025.
- **NR331 / NR331S** — Transport Department Schedule of Services (NR331en.pdf)
  for the exact Ma Wan departures; service hours for the circular return leg.
- **Fares** — PITCL bus service leaflets linked from `pitcl.com.hk/en/bus_service.php`,
  effective 1 April 2025 (NR338S effective 1 September 2025).
- **Ferries** — PITCL ferry timetables effective 17 August 2025, linked from
  `pitcl.com.hk/en/ferry_service.php`.
- **Weather** — Hong Kong Observatory Open Data API (`rhrread`, `warnsum`,
  `flw`), selecting the stations and rainfall districts nearest Ma Wan.
- **Traffic** — Transport Department Special Traffic News XML. The feed is
  territory-wide, so messages are flagged by whether they mention Ma Wan, the
  ferry piers, the airport corridor, Tsing Yi/Kwai Fong/Tsuen Wan or a
  connecting road. Railway-only notices are excluded from the default view.
- **News** — the list and detail pages under `pitcl.com.hk/en/news.php` are
  parsed as HTML; there is no JSON feed. PITCL's TLS certificate does not match
  the hostname, so those read-only requests accept the certificate.
- **Guidance** — PITCL's contact page, the high-wind/typhoon contingency PDF
  (PAX Notice 2018.08) and the ferry Notes to Passengers.
- **Official stops** — route notes flag where the live feed covers fewer stops
  than the operator's PDF, notably NR334 HZMB/Terminal 1–2 and NR338 Harbour Centre.

## Limits

- NR331, NR331S and NR338S have no public real-time feed. `mawan eta` says so
  instead of inventing a prediction.
- Weekend and public-holiday PITCL timetables are frequency windows; only the
  weekday timetable is published as exact clock times by the feed.
- The return legs of NR331/NR331S are circular services and are shown as
  frequencies because Sun Bus does not publish exact return times.
- The Socif feed labels some values `Scheduled` (the bus has not left the
  terminus yet). Those rows are marked `schedule`, not `live`.
- Ferries have no public real-time feed; they are timetable-only.
- Weather can suspend ferries and slow buses, but the tools report official
  warnings and forecasts; they do not predict a specific cancellation. In
  particular, T8 does not automatically cancel buses — read `pitcl_impact`.
- Traffic relevance is keyword-based. Use `mawan traffic --all` when a
  territory-wide incident may have knock-on effects not caught by the filter.
- The PITCL news page does not expose publication dates, so notices are shown
  in the site's own order (newest first) rather than with a parsed date.
- The adverse-weather plan is PITCL's published 2018 contingency notice; it is
  still linked from the site, but actual arrangements can change. Always check
  current PITCL notices and HKO warnings.
- `mawan guide` and the `pitcl_impact` text are English-only; the live weather,
  traffic and notice commands support `--lang tc`.
- Feeds can change without notice. This is an independent, unofficial project,
  not affiliated with PITCL, Sun Bus, KMB or the Transport Department.
