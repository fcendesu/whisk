# Whisk — Backend Hosting, Contracts and Reliability — Design

**Scope:** One-node Docker Compose, Axum API, SQLite migrations, OpenAPI 3.1, WS gateway and operational lifecycle. References: `../../architecture.md`, `../../deployment.md`, `../../api.md`, `../../events.md`.

## Behavior

Backend initializes a unique instance ID and one default community. `/api/v1/instance` exposes safe capability/version metadata for native client handshake. HTTP endpoints own writes and reads, and durable WS event rows are stored transactionally alongside data mutations. The server must stay usable for text when LiveKit or object storage is degraded.

## Design choice

Use a Rust workspace with `services/server`, `crates/protocol` and a storage crate. SQLite WAL and SQLx migrations are sufficient for single-node friend communities; PostgreSQL/HA remains a future extension. Deploy an API container, LiveKit, TLS termination; private local volume default and optional S3 profile. Pin versions after feasibility testing.

## Failure modes and tests

Boot fails on invalid config without leaking secrets; single-use owner bootstrap; database and object storage readiness; Docker restart keeps files and messages; server identity mismatch warns the client; `/sync` handles private-event gaps and cursor expiration. Record media UDP/TURN diagnostics; HTTP reverse proxy alone cannot guarantee voice reachability.

## Alternatives

Distributed microservices + PostgreSQL + Redis: excessive for v1; direct LiveKit-as-chat-store: lacks independent persistence/role model.
