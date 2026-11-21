# syntax=docker/dockerfile:1

FROM rust:1-bookworm AS build
WORKDIR /src
COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN cargo build --locked --release --bin mawan

FROM debian:bookworm-slim
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --create-home --uid 10001 mawan

COPY --from=build /src/target/release/mawan /usr/local/bin/mawan

ENV MAWAN_CACHE_DIR=/tmp/mawan-cache
USER mawan
EXPOSE 8788

# Set MAWAN_MCP_API_KEY to require a bearer token / X-API-Key on /mcp.
ENTRYPOINT ["mawan"]
CMD ["mcp", "--transport", "http", "--host", "0.0.0.0", "--port", "8788"]
