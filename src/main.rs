use std::collections::BTreeMap;
use std::process::ExitCode;
use std::time::Duration;

use anyhow::{Result, anyhow};
use clap::{Parser, Subcommand};
use ma_wan_bus::mcp::{self, McpArgs};
use ma_wan_bus::model::{parse_day, parse_day_with_date};
use ma_wan_bus::render::{self, Lang};
use ma_wan_bus::service::{self, Service};
use serde_json::json;

#[derive(Parser)]
#[command(
    name = "mawan",
    version,
    about = "Ma Wan (Park Island) buses, ferries, weather, traffic and PITCL guidance — CLI and MCP server",
    arg_required_else_help = true
)]
struct Cli {
    /// Print machine-readable JSON
    #[arg(long, global = true)]
    json: bool,

    /// Language for text output: en or tc
    #[arg(long, global = true, default_value = "en")]
    lang: String,

    /// Cache API responses for N seconds (0 disables)
    #[arg(long, global = true, default_value_t = 30)]
    cache_ttl: u64,

    /// Ignore cached responses for this run
    #[arg(long, global = true)]
    refresh: bool,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// List every Ma Wan bus route and whether live data is available
    Routes,
    /// Show one route, both directions, and its stops
    Route {
        /// Route code, e.g. NR330, NR331, 230R
        route: String,
    },
    /// List stops for one direction
    Stops {
        /// Route code
        route: String,
        /// 1/outbound or 2/inbound (default 1)
        #[arg(long, short)]
        direction: Option<String>,
    },
    /// Real-time arrivals, optionally filtered to one stop
    #[command(alias = "arrivals")]
    Eta {
        /// Route code
        route: String,
        /// 1/outbound or 2/inbound (default 1)
        #[arg(long, short)]
        direction: Option<String>,
        /// Stop sequence number or part of the stop name
        #[arg(long, short)]
        stop: Option<String>,
        /// Arrivals to show per stop (default 3)
        #[arg(long, short, default_value_t = 3)]
        limit: usize,
    },
    /// Published timetable for a route and day type
    Schedule {
        /// Route code
        route: String,
        /// 1/outbound or 2/inbound (default 1)
        #[arg(long, short)]
        direction: Option<String>,
        /// weekday, saturday, sunday, holiday, today, tomorrow, or YYYY-MM-DD
        #[arg(long, default_value = "today")]
        day: String,
    },
    /// Latest notices from the PITCL website
    News {
        /// Notice id to read in full (from `mawan news`)
        #[arg(long)]
        id: Option<u32>,
        /// Number of notices to list (default 10)
        #[arg(long, short, default_value_t = 10)]
        limit: usize,
    },
    /// Park Island ferry routes, fares and published timetables
    Ferry {
        #[command(subcommand)]
        command: FerryCommand,
    },
    /// Current weather, active warnings and forecast from the Hong Kong Observatory
    Weather,
    /// Special traffic news from the Transport Department
    Traffic {
        /// Show all territory-wide messages, not just Ma Wan-related ones
        #[arg(long)]
        all: bool,
        /// Messages to show (default 10)
        #[arg(long, short, default_value_t = 10)]
        limit: usize,
    },
    /// PITCL contact details, adverse-weather plan and passenger policies
    Guide {
        #[command(subcommand)]
        command: GuideCommand,
    },
    /// Combined weather impact, relevant traffic and PITCL notices
    Status,
    /// Next buses and ferries from a chosen origin
    Next {
        /// park-island, ma-wan, central, tsuen-wan, tsuen-wan-west, tsing-yi, kwai-fong, airport, kowloon-station
        #[arg(long, default_value = "park-island")]
        from: String,
        /// weekday, saturday, sunday, holiday, today, tomorrow, or YYYY-MM-DD
        #[arg(long, default_value = "today")]
        day: String,
        /// Departures to show per service (default 3)
        #[arg(long, short, default_value_t = 3)]
        limit: usize,
    },
    /// Provenance and freshness of the static datasets
    DataInfo,
    /// Run the MCP server over stdio or streamable HTTP
    Mcp(McpArgs),
}

#[derive(Subcommand)]
enum GuideCommand {
    /// Hotline, email, address and resident fare registration
    Contact,
    /// High wind and typhoon arrangements for buses and ferries
    Weather,
    /// Ferry luggage, pet, fare and on-board rules
    Policies,
}

#[derive(Subcommand)]
enum FerryCommand {
    /// List ferry routes, piers and fares
    Routes,
    /// Published timetable for a ferry route and day type
    Schedule {
        /// Ferry route: central or tsuen-wan
        route: String,
        /// weekday, saturday, sunday, holiday, today, tomorrow, or YYYY-MM-DD
        #[arg(long, default_value = "today")]
        day: String,
    },
}

#[tokio::main]
async fn main() -> ExitCode {
    match run().await {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error:#}");
            ExitCode::FAILURE
        }
    }
}

async fn run() -> Result<()> {
    let cli = Cli::parse();
    let lang = Lang::parse(&cli.lang)
        .ok_or_else(|| anyhow!("--lang must be en or tc (got '{}')", cli.lang))?;

    let cache_ttl = if cli.refresh { 0 } else { cli.cache_ttl };
    if let Command::Mcp(args) = cli.command {
        return mcp::run(args, Duration::from_secs(cache_ttl)).await;
    }

    let service = Service::new(Duration::from_secs(cache_ttl))?;

    match cli.command {
        Command::Routes => {
            let routes = service.routes();
            if cli.json {
                println!("{}", serde_json::to_string_pretty(&routes)?);
            } else {
                print!("{}", render::routes_table(&routes, lang));
            }
        }
        Command::Route { route } => {
            let info = service.route(&route)?;
            let first = service.stops(&info.code, 1).await.unwrap_or_default();
            let second = service.stops(&info.code, 2).await.unwrap_or_default();
            if cli.json {
                let mut stops = BTreeMap::new();
                stops.insert("1", &first);
                stops.insert("2", &second);
                println!(
                    "{}",
                    serde_json::to_string_pretty(&json!({
                        "route": info,
                        "stops": stops,
                    }))?
                );
            } else {
                let name = match lang {
                    Lang::En => &info.name_en,
                    Lang::Tc => &info.name_tc,
                };
                println!("{}  {}", info.code, name);
                println!("{:<14}{}", "Operator:", info.operator);
                println!(
                    "{:<14}{}",
                    "Live data:",
                    if info.realtime { "yes" } else { "no" }
                );
                println!("{:<14}{}", "Source:", info.source);
                if let Some(fare) = &info.fare {
                    println!("{:<14}{}", "Fare:", render::fare_summary(fare));
                    if let Some(note) = &fare.note {
                        println!("{:<14}{}", "Fare note:", note);
                    }
                }
                for note in &info.notes {
                    println!("{:<14}{}", "Note:", note);
                }
                if let Some(gaps) = render::official_gaps(&info) {
                    println!("{:<14}{}", "Official gaps:", gaps);
                }
                for (direction, stops) in [("1", &first), ("2", &second)] {
                    println!("\nDirection {direction}");
                    print!("{}", render::stops_table(stops, lang));
                }
            }
        }
        Command::Stops { route, direction } => {
            let direction = service::parse_direction(direction.as_deref())?;
            let stops = service.stops(&route, direction).await?;
            if cli.json {
                println!("{}", serde_json::to_string_pretty(&stops)?);
            } else {
                print!("{}", render::stops_table(&stops, lang));
            }
        }
        Command::Eta {
            route,
            direction,
            stop,
            limit,
        } => {
            let direction = service::parse_direction(direction.as_deref())?;
            let etas = service.etas(&route, direction).await?;
            let etas = service::filter_stop(etas, stop.as_deref())?;
            let etas = service::limit_per_stop(etas, limit);
            if cli.json {
                println!("{}", serde_json::to_string_pretty(&etas)?);
            } else if etas.is_empty() {
                println!("No upcoming arrivals reported by the operator.");
            } else {
                print!("{}", render::etas_table(&etas, lang));
            }
        }
        Command::Schedule {
            route,
            direction,
            day,
        } => {
            let direction = service::parse_direction(direction.as_deref())?;
            let day = parse_day(&day).map_err(anyhow::Error::msg)?;
            let schedule = service.schedule(&route, direction, day).await?;
            if cli.json {
                println!("{}", serde_json::to_string_pretty(&schedule)?);
            } else {
                print!("{}", render::schedule_text(&schedule, lang));
            }
        }
        Command::News { id, limit } => {
            if let Some(id) = id {
                let detail = service.news_detail(lang.code(), id).await?;
                if cli.json {
                    println!("{}", serde_json::to_string_pretty(&detail)?);
                } else {
                    print!("{}", render::news_detail_text(&detail));
                }
            } else {
                let items = service.news(lang.code(), limit).await?;
                if cli.json {
                    println!("{}", serde_json::to_string_pretty(&items)?);
                } else {
                    print!("{}", render::news_list_text(&items, lang));
                }
            }
        }
        Command::Ferry { command } => match command {
            FerryCommand::Routes => {
                let routes = service.ferry_routes();
                if cli.json {
                    println!("{}", serde_json::to_string_pretty(&routes)?);
                } else {
                    print!("{}", render::ferry_routes_table(&routes, lang));
                }
            }
            FerryCommand::Schedule { route, day } => {
                let day = parse_day(&day).map_err(anyhow::Error::msg)?;
                let schedule = service.ferry_schedule(&route, day)?;
                if cli.json {
                    println!("{}", serde_json::to_string_pretty(&schedule)?);
                } else {
                    print!("{}", render::ferry_schedule_text(&schedule, lang));
                }
            }
        },
        Command::Weather => {
            let report = service.weather(lang.code()).await?;
            if cli.json {
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else {
                print!("{}", render::weather_text(&report, lang));
            }
        }
        Command::Traffic { all, limit } => {
            let report = service.traffic(lang.code(), all, limit).await?;
            if cli.json {
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else {
                print!("{}", render::traffic_text(&report, lang));
            }
        }
        Command::Guide { command } => {
            let topic = match command {
                GuideCommand::Contact => "contact",
                GuideCommand::Weather => "weather",
                GuideCommand::Policies => "policies",
            };
            let guide = service.guide(topic, lang.code())?;
            if cli.json {
                println!("{}", serde_json::to_string_pretty(&guide)?);
            } else {
                print!("{}", render::guide_text(&guide));
            }
        }
        Command::Status => {
            let report = service.status(lang.code()).await;
            if cli.json {
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else {
                print!("{}", render::status_text(&report, lang));
            }
        }
        Command::Next { from, day, limit } => {
            let (day, date) = parse_day_with_date(&day).map_err(anyhow::Error::msg)?;
            let report = service.next(&from, day, date, limit, lang.code()).await?;
            if cli.json {
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else {
                print!("{}", render::next_text(&report, lang));
            }
        }
        Command::DataInfo => {
            let datasets = service.datasets();
            if cli.json {
                println!("{}", serde_json::to_string_pretty(&datasets)?);
            } else {
                print!("{}", render::datasets_text(&datasets, lang));
            }
        }
        Command::Mcp(_) => unreachable!("handled above"),
    }
    Ok(())
}
