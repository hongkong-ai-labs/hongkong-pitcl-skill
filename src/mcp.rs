//! MCP server exposing Ma Wan buses, ferries, weather, traffic and PITCL guidance.

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result, bail};
use axum::Router;
use axum::routing::get;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{Implementation, ServerCapabilities, ServerConfig};
use rmcp::transport::stdio;
use rmcp::transport::streamable_http_server::{
    StreamableHttpServerConfig, StreamableHttpService, session::local::LocalSessionManager,
};
use rmcp::{ErrorData, ServerHandler, ServiceExt, tool, tool_handler, tool_router};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::json;
use tokio_util::sync::CancellationToken;
use tower_http::cors::CorsLayer;

use crate::model::parse_day;
use crate::service::{self, Service};

#[derive(Debug, Clone, Copy, clap::ValueEnum)]
pub enum McpTransport {
    Stdio,
    Http,
}

#[derive(Debug, Clone, clap::Args)]
pub struct McpArgs {
    /// Transport: stdio or http (streamable HTTP)
    #[arg(long, value_enum, default_value_t = McpTransport::Stdio)]
    pub transport: McpTransport,
    /// Bind address for the HTTP transport
    #[arg(long, default_value = "127.0.0.1")]
    pub host: String,
    /// Bind port for the HTTP transport
    #[arg(long, default_value_t = 8788)]
    pub port: u16,
    /// Streamable HTTP endpoint path
    #[arg(long, default_value = "/mcp")]
    pub path: String,
}

pub async fn run(args: McpArgs, cache_ttl: Duration) -> Result<()> {
    let service = Arc::new(Service::new(cache_ttl)?);
    match args.transport {
        McpTransport::Stdio => {
            eprintln!("mawan MCP server listening on stdio");
            let server = MawanServer::new(service);
            let running = server.serve(stdio()).await?;
            running.waiting().await?;
            Ok(())
        }
        McpTransport::Http => serve_http(args, service).await,
    }
}

async fn serve_http(args: McpArgs, service: Arc<Service>) -> Result<()> {
    if !args.path.starts_with('/') {
        bail!("--path must start with '/'");
    }
    let cancellation = CancellationToken::new();
    let config = StreamableHttpServerConfig::default()
        .with_cancellation_token(cancellation.child_token())
        .with_allowed_hosts([
            args.host.clone(),
            "localhost".to_string(),
            "127.0.0.1".to_string(),
            "0.0.0.0".to_string(),
        ]);
    let http_service = StreamableHttpService::new(
        {
            let service = service.clone();
            move || Ok::<_, std::io::Error>(MawanServer::new(service.clone()))
        },
        LocalSessionManager::default().into(),
        config,
    );
    let router = Router::new()
        .route("/health", get(|| async { "ok" }))
        .nest_service(&args.path, http_service)
        .layer(CorsLayer::permissive());
    let address: SocketAddr = format!("{}:{}", args.host, args.port)
        .parse()
        .with_context(|| format!("invalid bind address {}:{}", args.host, args.port))?;
    let listener = tokio::net::TcpListener::bind(address).await?;
    eprintln!(
        "mawan MCP server (streamable HTTP) listening on http://{address}{}",
        args.path
    );
    let shutdown = cancellation.clone();
    axum::serve(listener, router)
        .with_graceful_shutdown(async move {
            let _ = tokio::signal::ctrl_c().await;
            shutdown.cancel();
        })
        .await?;
    Ok(())
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct RouteOnlyArgs {
    /// Route code: NR330, NR332, NR334, NR338, NR338S, NR331, NR331S, or 230R
    pub route: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct RouteArgs {
    /// Route code: NR330, NR332, NR334, NR338, NR338S, NR331, NR331S, or 230R
    pub route: String,
    /// Direction: 1/outbound or 2/inbound. Defaults to 1.
    pub direction: Option<u8>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ArrivalsArgs {
    /// Route code: NR330, NR332, NR334, NR338, or 230R
    pub route: String,
    /// Direction: 1/outbound or 2/inbound. Defaults to 1.
    pub direction: Option<u8>,
    /// Stop sequence number, or part of the stop name in English or Chinese
    pub stop: Option<String>,
    /// Arrivals to show per stop. Defaults to 3.
    pub limit: Option<usize>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct NewsArgs {
    /// Number of notices to list. Defaults to 10.
    pub limit: Option<usize>,
    /// Notice id to read in full. Omit to list the latest notices.
    pub id: Option<u32>,
    /// Language: en or tc. Defaults to en.
    pub lang: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GuideArgs {
    /// Topic: contact, weather, or policies
    pub topic: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct WeatherArgs {
    /// Language: en or tc. Defaults to en.
    pub lang: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct TrafficArgs {
    /// Include all territory-wide messages, not just Ma Wan-related ones.
    pub all: Option<bool>,
    /// Messages to return. Defaults to 10; 0 means no limit.
    pub limit: Option<usize>,
    /// Language: en or tc. Defaults to en.
    pub lang: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct FerryScheduleArgs {
    /// Ferry route: central or tsuen-wan
    pub route: String,
    /// weekday, saturday, sunday, holiday, today, tomorrow, or YYYY-MM-DD
    pub day: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ScheduleArgs {
    /// Route code: NR330, NR332, NR334, NR338, NR338S, NR331, NR331S, or 230R
    pub route: String,
    /// Direction: 1/outbound or 2/inbound. Defaults to 1.
    pub direction: Option<u8>,
    /// weekday, saturday, sunday, holiday, today, tomorrow, or YYYY-MM-DD
    pub day: Option<String>,
}

#[derive(Clone)]
pub struct MawanServer {
    service: Arc<Service>,
}

fn mcp_error(error: impl std::fmt::Display) -> ErrorData {
    ErrorData::internal_error(error.to_string(), None)
}

fn lang_code(value: Option<&str>) -> Result<&'static str, ErrorData> {
    match value.unwrap_or("en") {
        "en" => Ok("en"),
        "tc" => Ok("tc"),
        other => Err(mcp_error(format!("lang must be en or tc (got '{other}')"))),
    }
}

fn pretty(value: &impl serde::Serialize) -> Result<String, ErrorData> {
    serde_json::to_string_pretty(value).map_err(mcp_error)
}

#[tool_router]
impl MawanServer {
    pub fn new(service: Arc<Service>) -> Self {
        Self { service }
    }

    #[tool(
        description = "List every bus route serving Ma Wan (Park Island): NR330, NR332, NR334, NR338, NR338S, NR331, NR331S and KMB 230R, with operator, termini, and whether a live ETA feed exists."
    )]
    async fn list_ma_wan_routes(&self) -> Result<String, ErrorData> {
        pretty(&self.service.routes())
    }

    #[tool(
        description = "Get one Ma Wan bus route with both directions, operator, data source, fares, notes, and the ordered stops in each direction."
    )]
    async fn get_ma_wan_route(
        &self,
        Parameters(args): Parameters<RouteOnlyArgs>,
    ) -> Result<String, ErrorData> {
        let info = self.service.route(&args.route).map_err(mcp_error)?;
        let first = self.service.stops(&info.code, 1).await.unwrap_or_default();
        let second = self.service.stops(&info.code, 2).await.unwrap_or_default();
        pretty(&json!({
            "route": info,
            "stops": { "1": first, "2": second },
        }))
    }

    #[tool(
        description = "List the ordered stops for one direction of a Ma Wan bus route. Direction 1 is the operator's outbound direction and 2 is inbound; for 230R, 1 is Kowloon Station → Ma Wan."
    )]
    async fn get_ma_wan_stops(
        &self,
        Parameters(args): Parameters<RouteArgs>,
    ) -> Result<String, ErrorData> {
        let direction = args.direction.unwrap_or(1);
        let stops = self
            .service
            .stops(&args.route, direction)
            .await
            .map_err(mcp_error)?;
        pretty(&stops)
    }

    #[tool(
        description = "Real-time estimated arrivals for a Ma Wan bus route (PITCL NR330/NR332/NR334/NR338 and KMB 230R only), with per-stop ETAs in Hong Kong time, minutes away, and whether each value is live or schedule-based. Filter by stop sequence or stop-name fragment. NR331/NR331S/NR338S have no public live feed."
    )]
    async fn get_ma_wan_arrivals(
        &self,
        Parameters(args): Parameters<ArrivalsArgs>,
    ) -> Result<String, ErrorData> {
        let direction = args.direction.unwrap_or(1);
        let etas = self
            .service
            .etas(&args.route, direction)
            .await
            .map_err(mcp_error)?;
        let etas = service::filter_stop(etas, args.stop.as_deref()).map_err(mcp_error)?;
        let etas = service::limit_per_stop(etas, args.limit.unwrap_or(3));
        pretty(&etas)
    }

    #[tool(
        description = "Published timetable for a Ma Wan bus route and day type (weekday, saturday, sunday, holiday, today, tomorrow, or YYYY-MM-DD). Returns exact departure times where the operator publishes them, otherwise frequency windows. All times are Hong Kong time."
    )]
    async fn get_ma_wan_schedule(
        &self,
        Parameters(args): Parameters<ScheduleArgs>,
    ) -> Result<String, ErrorData> {
        let direction = args.direction.unwrap_or(1);
        let day = parse_day(args.day.as_deref().unwrap_or("today")).map_err(mcp_error)?;
        let schedule = self
            .service
            .schedule(&args.route, direction, day)
            .await
            .map_err(mcp_error)?;
        pretty(&schedule)
    }

    #[tool(
        description = "Latest notices from the PITCL (Park Island Transport) website. Returns titles and links, newest first. Pass an id from the list to read that notice in full. Use lang 'tc' for Traditional Chinese. These are operator announcements: service changes, fares, weather and ferry/bus adjustments."
    )]
    async fn get_ma_wan_news(
        &self,
        Parameters(args): Parameters<NewsArgs>,
    ) -> Result<String, ErrorData> {
        let lang = lang_code(args.lang.as_deref())?;
        if let Some(id) = args.id {
            let detail = self
                .service
                .news_detail(lang, id)
                .await
                .map_err(mcp_error)?;
            return pretty(&detail);
        }
        let items = self
            .service
            .news(lang, args.limit.unwrap_or(10))
            .await
            .map_err(mcp_error)?;
        pretty(&items)
    }

    #[tool(
        description = "List Park Island ferry routes (Park Island ↔ Central Pier No. 2 and Park Island ↔ Tsuen Wan Pier) with piers and published fares. Ferries have no public real-time feed; use get_ma_wan_ferry_schedule for times."
    )]
    async fn list_ma_wan_ferries(&self) -> Result<String, ErrorData> {
        pretty(&self.service.ferry_routes())
    }

    #[tool(
        description = "Published Park Island ferry timetable for a day type (weekday, saturday, sunday, holiday, today, tomorrow, or YYYY-MM-DD). Returns sailings from Park Island Pier and from the other pier, plus fares. Times are pier gate-closing times in Hong Kong time; there is no real-time ferry feed."
    )]
    async fn get_ma_wan_ferry_schedule(
        &self,
        Parameters(args): Parameters<FerryScheduleArgs>,
    ) -> Result<String, ErrorData> {
        let day = parse_day(args.day.as_deref().unwrap_or("today")).map_err(mcp_error)?;
        let schedule = self
            .service
            .ferry_schedule(&args.route, day)
            .map_err(mcp_error)?;
        pretty(&schedule)
    }

    #[tool(
        description = "Current Hong Kong Observatory weather relevant to Ma Wan, plus a PITCL-specific impact assessment: temperatures at the nearest stations, humidity, rainfall, UV, active warnings (typhoon, rainstorm, thunderstorm, monsoon), the local forecast, and separate bus/ferry guidance. A T8 does not automatically cancel buses: ferries suspend about 2 hours after T8, while buses continue until Stage 2 high-wind management on Tsing Ma Bridge (>75 km/h)."
    )]
    async fn get_ma_wan_weather(
        &self,
        Parameters(args): Parameters<WeatherArgs>,
    ) -> Result<String, ErrorData> {
        let lang = lang_code(args.lang.as_deref())?;
        let report = self.service.weather(lang).await.map_err(mcp_error)?;
        pretty(&report)
    }

    #[tool(
        description = "Real-time Transport Department special traffic news. By default only messages matching the Ma Wan corridor, ferry piers, airport, Tsing Yi/Kwai Fong/Tsuen Wan or connecting districts are returned; set all=true for every territory-wide message. Each item includes headline, detail, districts, reference time and whether it is Ma Wan-related."
    )]
    async fn get_ma_wan_traffic(
        &self,
        Parameters(args): Parameters<TrafficArgs>,
    ) -> Result<String, ErrorData> {
        let lang = lang_code(args.lang.as_deref())?;
        let report = self
            .service
            .traffic(lang, args.all.unwrap_or(false), args.limit.unwrap_or(10))
            .await
            .map_err(mcp_error)?;
        pretty(&report)
    }

    #[tool(
        description = "Static PITCL guidance that is not in the live feeds. Topics: 'contact' for the hotline, email, address and resident Octopus registration; 'weather' for the official high-wind and typhoon contingency plan (Tsing Ma Bridge wind thresholds, bus suspension, ferry strengthening); 'policies' for ferry fares, luggage, pets and on-board rules. These are published policies, not live predictions — check get_ma_wan_weather and get_ma_wan_news for the current situation."
    )]
    async fn get_pitcl_guide(
        &self,
        Parameters(args): Parameters<GuideArgs>,
    ) -> Result<String, ErrorData> {
        let guide = self.service.guide(&args.topic).map_err(mcp_error)?;
        pretty(&guide)
    }
}

#[tool_handler(
    name = "mawan",
    version = "0.1.0",
    instructions = "Ma Wan (Park Island) transport: buses, ferries, PITCL notices, weather and traffic. Live bus data covers PITCL NR330/NR332/NR334/NR338 and KMB 230R; published timetables cover NR338S, NR331 and NR331S. Ferries (Central and Tsuen Wan) are timetable-only. Use get_ma_wan_weather for HKO warnings and forecast (they can suspend ferries or slow buses), get_ma_wan_traffic for TD special traffic news, get_ma_wan_news for operator announcements, and get_pitcl_guide for static contact, adverse-weather and passenger-policy information. All times are Hong Kong time (UTC+8); never describe static timetables as live."
)]
impl ServerHandler for MawanServer {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(
            ServerCapabilities::builder().enable_tools().build(),
        )
        .with_server_info(Implementation::new("mawan", "0.1.0"))
        .with_instructions(
            "Ma Wan (Park Island) buses, ferries, PITCL notices, HKO weather and TD traffic. Live buses: PITCL NR330/NR332/NR334/NR338 and KMB 230R. Timetable only: NR338S, NR331, NR331S and both ferry routes. Times are Hong Kong time (UTC+8). Use get_pitcl_guide for contact details, typhoon/high-wind plans and passenger rules.",
        )
    }
}
