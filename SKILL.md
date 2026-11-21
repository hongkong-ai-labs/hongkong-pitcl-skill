---
name: ma-wan-bus
description: Live and scheduled transport for Ma Wan / Park Island, Hong Kong — PITCL buses NR330, NR332, NR334, NR338, NR338S; Sun Bus NR331 and NR331S; KMB 230R; Park Island ferries to Central and Tsuen Wan; fares; HKO weather warnings and forecasts; TD special traffic news; PITCL notices, contact details, typhoon/high-wind plans and passenger policies; combined disruption status, next departures and dataset freshness. Use for real-time arrivals, timetables, ferry sailings, stops, fares, weather, traffic, service changes, lost property, luggage/pet rules, or any question about getting to or from Ma Wan.
license: MIT
---

# Ma Wan transport

`mawan` is a Rust CLI and MCP server for Ma Wan (Park Island) buses, ferries
and PITCL notices. The CLI and stdio MCP server need no API key or account. The
HTTP MCP transport accepts an optional static API key. It is an independent,
unofficial project.

Use it when the user asks about:

- the next bus at Park Island, Ma Wan, Tsing Yi, Kwai Fong, the Airport,
  Central, Tsuen Wan, Tsuen Wan West, Kowloon Station, or Noah's Ark
- route NR330, NR332, NR334, NR338, NR338S, NR331, NR331S or 230R
- the ferry to Central Pier No. 2 or Tsuen Wan Pier
- fares, including resident, child and elderly fares
- weather warnings or forecasts that may suspend ferries or slow buses
- road incidents, closures or special traffic arrangements near Ma Wan
- the latest service changes, weather arrangements or fare notices
- PITCL's hotline, email, address or resident Octopus registration
- luggage, pet or on-board rules for the ferry
- whether a route has real-time data or only a timetable
- first/last buses, weekday vs Saturday/Sunday/public-holiday times
- stops and direction of travel, including official stops missing from live predictions
- the next bus or ferry from Park Island, Ma Wan, Central, Tsuen Wan or the town-side termini
- a combined disruption status from weather, traffic and PITCL notices
- how fresh a fare, timetable or policy is

## Before first use

Run `scripts/ensure-binary.sh` from the skill directory:

```console
$ bash scripts/ensure-binary.sh
/Users/you/.local/bin/mawan
```

It prints the absolute path of a working binary. Use that path for every
command, for example:

```bash
BIN="$(bash scripts/ensure-binary.sh)"
```

It checks, in order: `$MAWAN_BIN`, `mawan` on `PATH`, then
`cargo build --release --locked` in the repository.

If only the skill files were installed (for example through `npx skills add`),
there may be no checkout to build from. In that case install the binary first
with `cargo install --path .` from a clone, or point `MAWAN_BIN` at a built
`mawan` binary.

## CLI

Prefer `--json` for anything you need to parse. Tables are for humans.
All times are Hong Kong time (UTC+8); JSON timestamps are RFC 3339 with
`+08:00`.

```bash
# All routes and what data each has
mawan routes --json

# One route, both directions, all stops
mawan route NR330 --json

# Stops for a direction (1 = outbound, 2 = inbound)
mawan stops NR330 --direction 1 --json
mawan stops 230R --direction 2 --json

# Live arrivals
mawan eta NR330 --json
mawan eta NR330 --direction 2 --stop "Tsing Yi" --limit 3 --json
mawan eta 230R --direction 2 --json

# Timetables
mawan schedule NR330 --day weekday --json
mawan schedule NR330 --day saturday --json
mawan schedule 230R --day today --json
mawan schedule NR338S --day 2026-12-25 --json

# Ferries (no real-time feed)
mawan ferry routes --json
mawan ferry schedule central --day weekday --json
mawan ferry schedule tsuen-wan --day sunday --json

# PITCL notices, newest first
mawan news --limit 10 --json
mawan news --id 61 --json
mawan news --lang tc --limit 5 --json

# Weather and traffic
mawan weather --json
mawan weather --lang tc --json
mawan traffic --json          # Ma Wan-related TD messages
mawan traffic --all --json    # every territory-wide TD message

# Combined status and next departures
mawan status --json
mawan next --from park-island --limit 3 --json
mawan next --from ma-wan --json
mawan next --from central --day tomorrow --json

# Static PITCL guidance and data freshness
mawan guide contact --json
mawan guide weather --json     # high wind / typhoon plan
mawan guide policies --json    # ferry luggage, pets, on-board rules
mawan data-info --json
mawan update --check
```

`--refresh` bypasses the response cache for one run.

Direction `1` is the operator's outbound direction and `2` is inbound. For
230R, `1` is Kowloon Station → Ma Wan.

`--day` accepts `weekday`, `saturday`, `sunday`, `holiday`, `today`,
`tomorrow`, or `YYYY-MM-DD`.

## What is live

| Route | Live ETA | Timetable |
| --- | --- | --- |
| NR330, NR332, NR334, NR338 | yes | yes |
| 230R (KMB, not PITCL) | yes | yes |
| NR338S | **no** | exact published times |
| NR331, NR331S | **no** | exact from Ma Wan; frequency for the return leg |

Ferries: **no live feed** for either Central or Tsuen Wan. Use
`mawan ferry schedule` and call the times published sailings, not arrivals.

Never describe NR331, NR331S, NR338S or ferry sailings as live. `mawan eta`
returns a clear error for them. Use the schedule commands instead.

Weather comes live from the Hong Kong Observatory; `get_ma_wan_weather`
returns active warnings, nearest-station temperatures, rainfall, regional wind
and the local forecast, plus a `pitcl_impact` object with separate bus and
ferry guidance. Traffic comes live from the Transport Department. The default
traffic view is filtered to Ma Wan-related messages; `--all` returns every
territory-wide message. A weather warning or road closure is an official
advisory, not a guarantee that a specific sailing or bus is cancelled.

`mawan status` combines those feeds with the latest PITCL notices. It says
explicitly that no public PITCL real-time service-status feed exists. Use it
before answering "are ferries running?" or "is anything disrupted?".

`mawan weather` and `mawan status` apply the same optional TD bridge-stage
extraction, so both report an official Stage I/II/III when one is published.

`mawan route <code> --json` includes route geometry where the operator publishes
it: `shape` is the encoded polyline and `geometry` is a GeoJSON `LineString`
with `[longitude, latitude]` coordinates. Do not synthesise a route or a shape
when those fields are absent; say the operator does not publish geometry.

`mawan next` combines live bus ETAs, exact timetables and ferry sailings. Use
`--from park-island`, `ma-wan`, `central`, `tsuen-wan`, `tsuen-wan-west`,
`tsing-yi`, `kwai-fong`, `airport` or `kowloon-station`. Ferry times are
timetable values, never live.

Do **not** assume T8 means every bus is cancelled. PITCL's published plan says
ferries suspend about 2 hours after T8, while buses may continue until Stage 2
high-wind management on Tsing Ma Bridge (>75 km/h) suspends them. At T3,
ferries may be limited or suspended but buses normally continue. The impact
text follows `--lang tc`.

HKO's open regional wind feed has **no Tsing Ma Bridge anemometer**. The tool
returns Tsing Yi as a regional proxy and sets `bridge_wind.available` to
`false`. When TD publishes a Tsing Ma Bridge high-wind traffic notice,
`bridge_wind.stage` is the official Stage I/II/III and `stage_source` says so.
Otherwise the stage is `unknown`. Never apply the 40/60/75 km/h thresholds from
the proxy alone, and never derive Stage 1/2 from wind speed.

Weekday PITCL timetables are exact clock times. Saturday, Sunday and public
holiday PITCL timetables are frequency windows, because that is how the
operator's feed publishes them. 230R is always frequency-based.

## Reading arrivals

Each `Eta` object has:

| Field | Meaning |
| --- | --- |
| `stop_seq` | stop order in that direction |
| `stop_en`, `stop_tc` | stop name |
| `direction` | 1 or 2 |
| `minutes` | minutes away as published |
| `at` | arrival time, RFC 3339 in UTC+8 |
| `realtime` | true when the operator is predicting, false when scheduled |
| `remark_en`, `remark_tc` | operator note, e.g. `Scheduled`, `Final Bus` |

A `Scheduled` remark means the bus has not left the terminus yet; the time is
a timetable estimate, not vehicle tracking. Say "scheduled" rather than
"live" for those rows.

`mawan route <code> --json` includes a `fare` object with adult, child and
elderly fares where the operator publishes them, plus a note about resident
Octopus fares. NR331/NR331S now carry Sun Bus fares. Ferries return the same
fare shape from `mawan ferry routes --json`, plus journey minutes and pier
addresses.

Each route also carries `official_stops` per direction. `live_name_en` and
`live_name_tc` show the matching live-feed name, or `null` when the stop is
published but absent from live predictions. NR334's HZMB call and separate
Terminal 1/2 stops are the main live-feed gaps.

`mawan data-info --json` lists the source, printed effective date where one
exists, and the last-verified date for every static dataset.

PITCL notices are listed newest first with links to the official page; the
site does not expose publication dates, so do not invent one. Read a notice
in full with `mawan news --id <id>` (add `--lang tc` for Chinese).

## MCP

```bash
mawan mcp                          # stdio, no auth
mawan mcp --transport http --port 8788
mawan mcp --transport http --port 8788 --api-key "$SECRET"
```

When a key is configured, send `Authorization: Bearer <key>` or
`X-API-Key: <key>` to `/mcp`. `/health` stays public. The key can also come
from `MAWAN_MCP_API_KEY` or `MAWAN_API_KEY`; `--auth-token` is an alias.

Tools: `list_ma_wan_routes`, `get_ma_wan_route`, `get_ma_wan_stops`,
`get_ma_wan_arrivals`, `get_ma_wan_schedule`, `get_ma_wan_news`,
`list_ma_wan_ferries`, `get_ma_wan_ferry_schedule`, `get_ma_wan_weather`,
`get_ma_wan_traffic`, `get_ma_wan_status`, `get_ma_wan_next_departures`,
`get_ma_wan_data_info`, `get_pitcl_guide`.

## Cautions

- Schedules change on public holidays and during typhoon/rainstorm
  arrangements. For a disruption, check the operator's notice rather than
  assuming the timetable holds.
- NR330A is a Sun Bus peak supplement under the NR330 corridor; it is not a
  separate timetable here.
- NR331/NR331S return times are circular-service frequencies, not exact times.
- Ferry times are pier gate-closing times, not departure estimates.
- Weather and traffic are official feeds, but do not infer that a specific
  ferry or bus is cancelled from a general warning or incident.
- The typhoon/high-wind plan is PITCL's published 2018 contingency notice.
  Present it as published guidance, not as the current operational status.
- `guide policies` covers the ferry Notes to Passengers; the bus notes PDF is
  image-only, so link to it rather than inventing bus rules.
- Do not infer Tsing Ma Bridge Stage 1/2 from the Tsing Yi wind proxy; HKO has
  no public bridge anemometer. `bridge_wind.available` is false by design. Use
  `bridge_wind.stage` only when TD has published an official high-wind notice.
- Do not invent route geometry. Report `geometry`/`shape` only when present.
- Traffic relevance is keyword-based; use `--all` if an incident may have
  knock-on effects outside the Ma Wan corridor.
- `mawan next` does not invent times for frequency-only return legs; report the
  note and point to `mawan schedule` instead.
- PITCL notices have no machine-readable dates; report them in site order and
  do not invent a date.
- Do not present this data as official; it is an independent client of public
  operator feeds.
