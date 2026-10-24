# Whisk — Messaging, Synchronization and File Storage — Design

**Scope:** channels/categories, DMs, replies/reactions, unread, offline replay, attachments. References: `../../database.md`, `../../api.md`, `../../events.md`, `../../deployment.md`.

## Write/read models

HTTP writes persist the resource and durable event in one transaction. Every message create includes `client_nonce`, stored under a unique (author, conversation, nonce) index. Clients show pending messages but replace them with the original committed record upon ack/replay. `/sync` or WS resume uses an instance cursor; private event gaps are expected, filtered by current access. Message pagination has stable cursors and 100-result maximum.

## File design

25 MiB limit (26,214,400 bytes) checked at intent and stream. Objects remain private; object keys are randomly generated and provider-independent. Storage adapter exposes `put`, `get_range`, `stat`, `delete`; local files default and S3-compatible RustFS/R2 optional. Finalize after verifying content size/hash and permissions. Metadata and message linkage stay in SQLite; support later backend migration by copying/verifying keys.

## Threats, failures and alternatives

Malicious filenames, MIME mismatch, active HTML, image decompression bombs, oversized chunked bodies, quota races, orphan uploads, unauthorized old URLs, stale client read state and replay duplication. Prefer backend-proxied authenticated 25 MiB transfers to direct presigned S3 URLs in v1. DMs are one-to-one within one instance only; do not merge across instances.
