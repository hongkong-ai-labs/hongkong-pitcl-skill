# Cloudflare deployment

This directory contains a Worker front door for the Rust HTTP MCP server.

Cloudflare runs the server in a **Cloudflare Container**, not as a pure
`wasm32-unknown-unknown` Worker. The server needs TCP/TLS and a persistent Tokio
runtime; rewriting the MCP tools and HTTP clients for a V8 isolate would
duplicate the Rust implementation.

The Worker is the public endpoint. It starts the container, probes `/health`,
and forwards `/mcp` and `/health` to it.

## Deploy

Prerequisites:

- a Cloudflare account with Containers enabled;
- Node.js and `npx`;
- Docker (or a Docker-compatible CLI) running locally, because Wrangler builds
  the `linux/amd64` image from the repository root `Dockerfile`;
- `npx wrangler login`.

From the repository root:

```bash
npx wrangler secret put MAWAN_MCP_API_KEY
npx wrangler deploy
```

`MAWAN_MCP_API_KEY` is optional. When set, `/mcp` requires
`Authorization: Bearer <key>` or `X-API-Key: <key>`; `/health` stays public.
Without it the endpoint is unauthenticated, so put Cloudflare Access or OAuth
in front of a public deployment if you need per-user identity.

Cloudflare builds the image, pushes it to the Cloudflare registry, and deploys
the Worker. The MCP endpoint is:

```text
https://<worker-name>.<account>.workers.dev/mcp
```

Connect an HTTP MCP client:

```bash
claude mcp add --transport http mawan \
  https://<worker-name>.<account>.workers.dev/mcp
```

## Local Worker check

`npx wrangler dev` also needs a working Docker engine because it starts the
container locally:

```bash
npx wrangler dev --port 8787
```

Use `http://localhost:8787/mcp` as the Worker MCP URL. Port 8788 is the
container's internal port and is not exposed directly.

## Transport

Cloudflare's current remote MCP transport is **Streamable HTTP** at `/mcp`.
Legacy SSE is deprecated and is not exposed here. The Rust server's `/mcp`
endpoint supports both JSON responses and SSE-framed responses.

## Pure WASM Worker status

A pure Rust Cloudflare Worker is not included. It requires replacing
`reqwest`/rustls, Tokio's multi-thread runtime and the stdio/HTTP MCP transport
with Workers-specific APIs. If a true V8-isolate Worker becomes a hard
requirement, the correct next step is a separate Worker crate with a `fetch`
adapter and rewritten HTTP backend, not this container.
