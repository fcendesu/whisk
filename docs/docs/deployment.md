# Whisk — Docker-first single-node deployment design

Whisk Server runs on a Linux server using Docker Compose. Whisk Desktop is the native client; Whisk Media refers to the LiveKit deployment/integration. Service names such as `api`, `livekit` and `caddy` are internal implementation details, not product rebrands.

Production backend runs on a Linux server using Docker Compose. The Windows/Linux/macOS requirement applies to the **desktop client**, not to hosting the server on every platform.

## Default Compose services

- `api`: Rust/Axum application plus database migrations; SQLite WAL metadata file in persistent volume.
- `livekit`: self-hosted SFU. UDP and TURN-related networking must be externally reachable as configured.
- `caddy`: reverse proxy and HTTPS termination for the API and instance discovery.
- Persistent `app-data` volume includes SQL and filesystem attachments; keep backup procedure consistent.
- Optional `rustfs` profile for S3-compatible private storage; optional Redis profile if media deployment warrants it. Do not force optional infrastructure on default setup without measuring need.

## Operator journey

1. Copy `.env.example` to `.env`, generate non-default secrets and set the canonical HTTPS base URL.
2. Configure LiveKit WebRTC UDP port range and TURN/public IP strategy for the actual hosting network. A normal HTTP tunnel is **not** a substitute for all media UDP traffic.
3. `docker compose -f deploy/compose.yaml up -d`; check `docker compose -f deploy/compose.yaml ps` and `/health/ready`.
4. Run single-use owner/bootstrap command. Backend creates exactly one default community, then disables one-time setup token.
5. Create first invitation, connect native client, validate text + voice + stream outside localhost.
6. For upgrades: take consistent database/object backup first, pin image tags/digests, run migrations, monitor readiness and restore if necessary.

## Storage adapters

`StorageBackend::{put, get_range, stat, delete}` implemented by `LocalFilesystemStore` and `S3Store`. For S3 set endpoint, bucket, region, path-style mode and credentials. Support RustFS, Cloudflare R2 and AWS S3 using `PutObject`, `GetObject`, `HeadObject`, `DeleteObject`; no assumption of public ACLs, ETag-as-content-SHA256, or provider URL persistence.

Default file limit is exactly **25 MiB = 26,214,400 bytes**, validated during initiation and streaming. Store every object under server-generated key with private access and a canonical digest.

## Migration Runbook: local/RustFS → R2 or other S3

1. Decide and communicate write freeze.
2. Back up SQLite *and* attachment bytes; record original key count and per-object SHA-256.
3. Copy objects without changing their keys or database attachment IDs.
4. Verify every object size and SHA-256; test GET and ranged GET through backend authorization.
5. Change configured adapter; smoke-test private attachment access and message references.
6. Keep old store intact through rollback window. Do not delete until acceptance.

## Observability

Expose liveness/readiness, structured logs with request IDs but no tokens or PII-bearing contents, media port diagnostics, storage readiness and migration version. Add a backup/restore smoke test in release CI. Record advertised public IP and actual UDP reachability in operator diagnostics.
