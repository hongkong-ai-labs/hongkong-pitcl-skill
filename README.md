# mawan

**Ma Wan / Park Island transport in one binary — CLI and MCP server.**

`mawan` pulls the live feeds behind the Park Island ETA app and Hong Kong's
open bus data, pairs them with the operators' published timetables, ferries,
fares and notices, and exposes everything over a CLI and MCP.

- **Live bus ETA:** PITCL NR330, NR332, NR334, NR338; KMB 230R
- **Bus timetables:** NR330, NR332, NR334, NR338, NR338S, NR331, NR331S, 230R
- **Fares:** published adult, child and elderly fares, including Sun Bus NR331/NR331S
- **Ferries:** Park Island ↔ Central Pier No. 2 and Park Island ↔ Tsuen Wan, with journey times and pier addresses
- **Status:** one call combining weather impact, relevant TD traffic and PITCL notices
- **Next departures:** next bus and ferry from Park Island, Ma Wan or the town-side termini
- **Weather:** current HKO conditions, active warnings, local forecast and regional wind
- **Traffic:** real-time TD special traffic news, filtered to the Ma Wan corridor
- **PITCL news:** latest operator notices in English or Traditional Chinese
- **PITCL guidance:** hotline, typhoon/high-wind plan and passenger policies, in English or Chinese
- **Data freshness:** effective dates and last-verified dates for every static dataset
- **Stops and directions** for every bus route, including structured official stop lists
- **Route geometry:** the operator's published shape as an encoded polyline and a GeoJSON LineString
- **Tsing Ma Bridge stage:** Stage I/II/III only when TD publishes a high-wind traffic notice
- **MCP server** over stdio and streamable HTTP, with an optional API key
- **Agent skill** (`SKILL.md`) plus a binary bootstrap script

No API key is needed for the CLI or stdio MCP server. The HTTP transport accepts
an optional static key. Times are Hong Kong time (UTC+8).

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
| NR331 | HK$15.60 | HK$9.60 | Sun Bus flat fare; child same as adult; section fare $2.00 within Ma Wan |
| NR331S | HK$16.30 | HK$10.00 | Sun Bus flat fare; child same as adult; section fare $2.00 within Ma Wan |
| 230R | HK$15.50 | — | KMB fare |

Use `mawan route <code>` for the full fare object, including child and
elderly fares. Sun Bus fares require a registered Octopus card for the
resident rate; the TD sheet does not print an effective date, so
`mawan data-info` records the last verification date instead.

## Ferries

| Route | Other pier | Journey | Weekday sailings | Weekend/holiday sailings |
| --- | --- | --- | --- | --- |
| `central` | Central Pier No. 2, Man Kwong Street | about 22 min | 19 each way | 7 each way |
| `tsuen-wan` | Tsuen Wan Ferry Pier | about 12 min | 3 each way | 3 each way |

Boarding is at Park Island Ferry Pier, 8 Pak Lai Road, Ma Wan. Ferry times are
pier gate-closing times. Fares: Central $36.00 adult visitor / $24.10 resident;
Tsuen Wan $12.00 / $10.20. Child and elderly fares are roughly half and are
returned in full by `mawan ferry routes --json`. `mawan ferry schedule` also
returns first/last sailings and the pier addresses.

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

The impact text, guide and headings follow `--lang tc`.

`mawan weather` also includes HKO's regional 10-minute wind observations.
There is **no public Tsing Ma Bridge anemometer** in the open-data feed. The
Tsing Yi station is shown as a regional proxy, but `bridge_wind.available` is
`false`, so PITCL's 40/60/75 km/h thresholds are never applied automatically.

When the Transport Department publishes a Tsing Ma Bridge high-wind traffic
notice, `bridge_wind.stage` is set from that notice and `stage_source` names
the source. This is the official stage, not an inference from nearby wind.
Without such a notice the stage is `unknown`.

## Install

**Agent skill** — installs `SKILL.md` plus the binary bootstrap script:

```bash
npx skills add hongkong-ai-labs/hongkong-pitcl-skill
```

**Prebuilt binary** — macOS (Intel or Apple silicon) and Linux (x86_64 or
aarch64, static musl), checksum-verified, into `~/.local/bin`:

```bash
curl -fsSL https://raw.githubusercontent.com/hongkong-ai-labs/hongkong-pitcl-skill/main/scripts/install.sh | sh
```

**From source** — Rust 1.89 or newer:

```bash
git clone https://github.com/hongkong-ai-labs/hongkong-pitcl-skill
cd hongkong-pitcl-skill
cargo install --path . --locked
# or: cargo build --release --locked   # target/release/mawan
```

**Docker / Apple container** — the same OCI image runs the CLI, stdio MCP or
HTTP MCP. See [Docker and Apple container](#docker-and-apple-container).

**Cloudflare** — `wrangler.jsonc` and `cloudflare/` deploy the HTTP server as a
Worker front door plus a Cloudflare Container; see
[Cloudflare Worker + Container](#cloudflare-worker--container). A pure WASM
Worker is not provided because the server needs TCP/TLS, a Tokio runtime and
bundled SQLite.

Verify the install:

```console
$ mawan --version
mawan 0.1.0
```

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

mawan weather                        # HKO conditions, warnings, forecast and wind
mawan weather --lang tc
mawan traffic                        # TD news affecting Ma Wan
mawan traffic --all --limit 10       # every territory-wide message

mawan status                         # weather impact + traffic + PITCL notices
mawan status --lang tc
mawan next                           # next buses and ferries from Park Island
mawan next --from ma-wan --limit 3   # NR331, NR331S, NR338S and 230R
mawan next --from central --day tomorrow

mawan guide contact                  # hotline, email, resident fare forms
mawan guide weather                  # high-wind and typhoon arrangements
mawan guide policies                 # ferry luggage, pets and on-board rules
mawan data-info                      # effective and last-verified dates
```

Global flags: `--json`, `--lang en|tc`, `--cache-ttl 30`, `--refresh`.
`--refresh` bypasses the in-process cache for that run.

MCP HTTP flags: `--transport http`, `--host 127.0.0.1`, `--port 8788`,
`--path /mcp`, `--api-key` (or `MAWAN_MCP_API_KEY`).

`mawan route <code>` always requests the operator shape. With `--json`, each
direction may include `shape` (the encoded polyline) and `geometry` (a GeoJSON
`LineString`). Routes without a published shape omit both fields and add a note;
the command never invents coordinates.

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
mawan mcp --transport http --port 8788 --api-key "$SECRET"
# or: MAWAN_MCP_API_KEY="$SECRET" mawan mcp --transport http
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
`GET /health` stays public. When an API key is configured, `/mcp` requires
`Authorization: Bearer <key>` (or `X-API-Key: <key>`); requests without it
receive HTTP 401. The server has no TLS of its own, so keep the loopback bind
or terminate TLS at a reverse proxy before exposing it.

```json
{
  "mcpServers": {
    "mawan": {
      "type": "http",
      "url": "http://127.0.0.1:8788/mcp",
      "headers": { "Authorization": "Bearer <key>" }
    }
  }
}
```

The key comes from `--api-key`, `MAWAN_MCP_API_KEY` or `MAWAN_API_KEY`; the
`--auth-token` spelling is accepted as an alias. An empty value disables auth.

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
| `get_ma_wan_weather` | HKO weather, warnings, forecast, regional wind and PITCL impact |
| `get_ma_wan_traffic` | TD special traffic news, Ma Wan-related by default |
| `get_ma_wan_status` | combined weather impact, traffic and PITCL notices |
| `get_ma_wan_next_departures` | next buses and ferries from a chosen origin |
| `get_ma_wan_data_info` | effective and last-verified dates for static data |
| `get_pitcl_guide` | contact, adverse-weather plan and passenger policies |

## Agent skill

```bash
npx skills add <owner>/hongkong-pitcl-skill
```

Then run `bash scripts/ensure-binary.sh` once. It checks `$MAWAN_BIN`, `mawan`
on `PATH`, and finally builds from the checkout, printing the absolute binary
path to use. `scripts/install.sh` prefers a checksum-verified GitHub release and
falls back to a source build.

### Docker and Apple container

```bash
docker build -t mawan .
docker run --rm mawan routes                       # CLI
docker run --rm -p 8788:8788 -e MAWAN_MCP_API_KEY="$SECRET" \
  mawan mcp --transport http --host 0.0.0.0 --port 8788
```

For stdio, keep stdin attached:

```bash
docker run --rm -i mawan mcp
```

`docker compose up --build` starts the HTTP server on port 8788. Apple's
`container` runtime uses the same OCI image:

```bash
container build -t mawan .
container run --rm -p 8788:8788 -e MAWAN_MCP_API_KEY="$SECRET" \
  mawan mcp --transport http --host 0.0.0.0 --port 8788
```

### Cloudflare Worker + Container

The repository includes a Worker front door in `cloudflare/` and
`wrangler.jsonc`. Cloudflare runs the Rust HTTP MCP server in a Cloudflare
Container, not as a pure WASM isolate, because the server needs TCP/TLS and a
Tokio runtime. Deploy from the repository root with
`npx wrangler deploy`; Wrangler needs a Docker-compatible engine to build the
image. See [`cloudflare/README.md`](cloudflare/README.md) for prerequisites and
authentication guidance.

## Development and releases

```bash
just                 # fmt check, clippy with -D warnings, tests
just skill-check     # validate SKILL.md and shell syntax
just container       # build the image and print CLI help
cargo test --all --locked
cargo clippy --all-targets --locked -- -D warnings
cargo fmt --all -- --check
```

CI runs the same checks. Dependabot proposes weekly Cargo and GitHub Actions
updates, and the audit workflow checks the lockfile against the RustSec
advisory database.

`just release 0.1.0` tags and pushes `v0.1.0`. The release workflow checks the
tag against `Cargo.toml` and `CHANGELOG.md`, builds four targets, smoke-tests
native binaries and publishes `mawan-<tag>-<target>.tar.gz` plus `SHA256SUMS`.
`scripts/install.sh` verifies the checksum and falls back to
`cargo build --release --locked`.

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
  weekday timetables and route `shape` values. Route IDs: NR330=249, NR332=250,
  NR334=252, NR338=253. The weekend/seasonal headways come from the same
  metadata. Signed `timestamp` values are converted to UTC+8. Shapes are decoded
  from the standard encoded-polyline format and emitted as GeoJSON LineStrings.
  `GET /api/route/{id}` now carries the shape; the separate
  `/api/route-network/` endpoint is not used because it currently returns a
  server-side `TypeError` and is unnecessary for per-route geometry.
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
  `flw`), selecting the stations and rainfall districts nearest Ma Wan. The
  regional 10-minute wind CSV supplies Tsing Yi, Chek Lap Kok, Cheung Chau,
  Central Pier and Waglan Island; it has no Tsing Ma Bridge station.
- **Traffic** — Transport Department Special Traffic News XML. The feed is
  territory-wide, so messages are flagged by whether they mention Ma Wan, the
  ferry piers, the airport corridor, Tsing Yi/Kwai Fong/Tsuen Wan or a
  connecting road. Railway-only notices are excluded from the default view.
  An explicit Tsing Ma Bridge high-wind management notice (Stage I/II/III, in
  English or Chinese) supplies `bridge_wind.stage` and `stage_evidence`.
  Territory-wide RTIS and speed-map pages are not used for the bridge stage:
  the documented XML is stable, while the speed-map endpoint has been returning
  HTTP 503 and its segment metadata does not cleanly identify Tsing Ma Bridge.
- **News** — the list and detail pages under `pitcl.com.hk/en/news.php` are
  parsed as HTML; there is no JSON feed. PITCL's TLS certificate does not match
  the hostname, so those read-only requests accept the certificate.
- **Guidance** — PITCL's contact page, the high-wind/typhoon contingency PDF
  (PAX Notice 2018.08) and the ferry Notes to Passengers.
- **Official stops** — every route/direction carries an `official_stops` list
  with a `live_name_en`/`live_name_tc` mapping. That makes differences
  machine-readable: NR334's HZMB call and separate Terminal 1/2 stops are the
  main live-feed gaps; NR338's Harbour Centre and Tin Liu calls are already in
  the inbound live list.
- **Freshness** — `mawan data-info` lists the source, printed effective date
  where one exists, and the date this project last verified each static dataset.

## Limits

- NR331, NR331S and NR338S have no public real-time feed. `mawan eta` says so
  instead of inventing a prediction. Their stop lists are the official TD/PITCL
  lists rather than live predictions.
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
- HKO publishes no Tsing Ma Bridge wind station in its open regional wind feed.
  The tool reports the nearest regional wind and leaves `bridge_wind.available`
  false. A bridge stage is reported only when TD publishes an official high-wind
  traffic notice; otherwise it is `unknown`. Stage 1/2 is never inferred from the
  Tsing Yi proxy.
- Route geometry exists only where the operator publishes a shape. It is
  returned as-is; missing geometry is reported, not synthesised.
- `mawan next` uses live ETAs where available and published times otherwise.
  Return legs published only as frequency windows (NR331/NR331S from town) are
  reported as notes, not invented departure times.
- Feeds can change without notice. This is an independent, unofficial project,
  not affiliated with PITCL, Sun Bus, KMB or the Transport Department.
