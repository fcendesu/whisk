# Whisk — v1 Product Requirements Document

**Document status:** Finalized product baseline (implementation pending)  
**Revision:** 1.0 • **Date:** 2026-10-24  
**Product category:** Native desktop communication for friends and small gaming communities  
**Working identity:** **Whisk** (user-selected working name; public trademark/domain clearance pending)  
**Primary audience:** Gaming friends, small self-hosted communities  
**Target release:** Windows, Linux (Wayland + X11), macOS  

> This is a v1 requirements baseline, not an assertion that unimplemented features or performance targets have been achieved. All gates marked *verify* must pass before release. The application is inspired by Discord and MicUp; it is **not positioned as a full Discord replacement**.

## 1. Executive summary and locked decisions

Build a responsive native Rust/GPUI desktop application to join independent, self-hosted communication instances. Each **backend instance** hosts **exactly one community in v1**; the backend data model allows many communities later. The **desktop client connects to multiple independent instances** and displays them in **one Discord-style unified sidebar**. Each instance's credentials, account, messages, permissions, storage and sync cursors remain separate. There is no central account, discovery service, or federation.

### Locked product decisions

| ID | Decision | Status |
|---|---|---|
| DEC-01 | Native Rust + GPUI UI, not Electron/browser-based | Locked |
| DEC-02 | Rust/Axum/Tokio backend; LiveKit SFU + Rust client SDK for realtime media | Locked baseline; integration feasibility gate |
| DEC-03 | Single community per independently deployed backend in v1 | Locked |
| DEC-04 | One GPUI app manages multiple independently authenticated instances in unified Discord-like sidebar | Locked |
| DEC-05 | One active *voice channel* across all instances per client; multiple text connections may remain active | v1 policy |
| DEC-06 | Invite-only account registration by default; operator may explicitly enable public signup | Locked |
| DEC-07 | Text, voice, screen/game streaming, DMs, attachments, webcam, advanced roles/moderation | Locked v1 scope |
| DEC-08 | Multiple simultaneous screen stream publishers per voice channel; viewers select one or more to watch (one focused stream first) | Locked |
| DEC-09 | 1080p60 and 1440p60 streaming supported on verified capable hardware; 720p30 fallback | Locked target, verify per OS/GPU |
| DEC-10 | Default maximum attachment size **25 MiB**, administrator-configurable | Locked |
| DEC-11 | Docker/Compose-ready backend deployment is a release requirement | Locked |
| DEC-12 | Storage backend: local filesystem by default; provider-independent S3-compatible adapter supports optional RustFS and Cloudflare R2 | Architecture decision; verify adapter contract |
| DEC-13 | License deliberately undecided pending dependency/license audit | Locked as open decision |
| DEC-14 | No cross-instance messaging or federation in v1 | Locked |
| DEC-15 | No mandatory proprietary cloud account or hosted service | Locked |
| DEC-16 | Product working name **Whisk**, inspired by using a kitchen whisk or wooden spatula as a pretend microphone | Locked name; brand availability unverified |

### Product identity and naming

- **Product name:** **Whisk**, chosen by the project owner on 2026-10-24.
- **Meaning:** A playful reference to holding a kitchen whisk (or wooden spatula) like a microphone while talking with friends. The name should feel approachable, not exclusively gaming-oriented.
- **Working product labels:** **Whisk Desktop** (native client), **Whisk Server** (self-hosted Rust backend), **Whisk Media** (LiveKit integration); **Whisk CLI** is only a possible future tool and not a v1 requirement.
- **Brand direction (not yet approved assets):** A restrained kitchen-whisk/microphone mark and an optional tagline, *“Your space. Your voice.”* Neither logo nor tagline is finalized by the user.
- **Legal/release gate:** Domain, repository naming, package identifiers, and trademarks have not been verified. Public release branding requires a collision/clearance review; selecting the name does **not** establish exclusive rights.
- Branding must not alter the instance/community distinction, protocol IDs, self-hostability, or feature scope.

**Assumptions chosen to unblock implementation:** single-node deployment; SQLite in WAL mode as default metadata database; per-instance usernames/passwords with Argon2id; all files stored privately; background notifications from multiple authenticated instances; no built-in call recording, server-side voice transcription, or end-to-end encryption claim. These are revisable ADRs, not user-confirmed demands.

## 2. Goals, non-goals and success definition

**User outcomes:** (G1) Deploy a private community on a single Linux host with Compose; (G2) connect and switch rapidly among several unrelated communities from one native app; (G3) chat reliably, including after network interruptions; (G4) join voice channels and stream games/screens or webcams with predictable permission control; (G5) administer invitations, channel access, members, roles, and files without touching the database.

**Non-goals in v1:** mobile, web client, cross-instance federation, worldwide instance directory, bots/marketplace, large broadcast audiences, in-DM live voice/video calls, livestream recording/egress, cross-community channels, remote-control streaming, elaborate emoji/sticker marketplace, distributed LiveKit or HA database, E2EE guarantees, community-per-instance count >1, automatic object-store replication.

**Product-level release success:** New users can complete administrator setup and invitation-based onboarding; a client can switch between ≥3 independent instances without data bleed; messages survive restart/disconnect and sync exactly once at UI level; voice, webcams, and multiple selectable concurrent screen streams function in a reference environment; moderation is consistently enforced; backup/restore and Docker deployment are documented and tested.

## 3. Terminology and tenancy

- **Instance:** Independent backend, base URL, stable server-generated instance UUID, separate database, storage and LiveKit deployment.
- **Community:** An organization hosted within an instance. Exactly one automatically provisioned community in v1, but database uses `community_id` for future extensibility.
- **Channel:** One text or one voice channel within community, ordered under optional category.
- **User:** Instance-local account; no global identity across instances.
- **Membership:** User's per-community role/permissions; in v1 all active users generally belong to sole community.
- **Client profile:** Local stored instance URL + instance ID + user ID + keychain credential reference + cache namespace + sync cursor.
- **Voice room:** LiveKit room mapped to one instance/community/voice channel.
- **DM thread:** Instance-local one-to-one conversation (independent of community membership if both accounts are allowed by policy).

IDs use opaque UUID/ULID identifiers as explicitly typed domain IDs. Clients must never treat identically formatted IDs across instances as globally interchangeable: use `(instance_id, entity_id)` internally. `instance_id` is checked upon connection; unexpected changes require user intervention. Enforce community scoping on server requests even though v1 has only one community.

## 4. Functional requirements and acceptance conditions

### 4.1 Instance management and accounts

**FR-ACC-01** Operator starts Compose and initializes one owner, instance name, public base URL and default community. Setup token is one-time; cannot recreate owners unauthenticated.  
**FR-ACC-02** Invite-only by default. Invite records include token hash, creator, role target, expiry, max uses, used count, disabled state. Admin can enable open registration explicitly.  
**FR-ACC-03** Password login/logout, session rotation, logout-all-devices, invalid/expired/used invite errors. Password hashes use Argon2id with calibrated parameters.  
**FR-ACC-04** Native client add/remove/rename instance profile, multiple profiles, Discord-style unified left rail, unread/mention badges per instance, independent notifications, connection status, quick switching.  
**FR-ACC-05** Credentials stored through OS secure credential store; SQLite cache per-instance/account namespace; remove profile optionally wipes local cache but not remote account.  
**FR-ACC-06** Background socket connections may stay open with resource budgets; only one voice channel active globally. Failure of one instance never locks others.  
**Accept:** 3 unrelated instances simultaneously connected with different accounts; no channel/file/message/token leakage; switching doesn't trigger forced re-login; server identity-change warning shown.

### 4.2 Channels, messaging and DMs

**FR-MSG-01** Create/edit/reorder/archive/delete text and voice channels/categories according to permissions.  
**FR-MSG-02** Send, edit, soft-delete, reply, basic emoji reactions and attachments; render Markdown-like safe subset and code blocks; never render untrusted HTML.  
**FR-MSG-03** Chronological history pagination with stable cursor, optimistic local sending, idempotent retries, unread state/read markers, per-channel new message badges and mentions.  
**FR-MSG-04** One-to-one DM threads between instance users; participant-only access, block users, DM unread counters, attachment support. No cross-instance DMs.  
**FR-MSG-05** Persistent per-instance event cursor, gap detection and API backfill. Events are at-least-once delivered; UI de-duplicates by server event ID and message ID.  
**FR-MSG-06** Server validates channel visibility, current membership, timeouts and file permissions on every mutating request; direct storage access bypass is impossible.  
**Accept:** During a 2-minute offline interval, missed messages, edits, deletes, channel changes and permission changes sync correctly on reconnect without duplicated visible messages or stale access.

### 4.3 Voice, screen streaming and webcam

**FR-MEDIA-01** Users join/leave voice channels, select input/output devices, see participants, mute/deafen, set per-participant playback gain, choose push-to-talk or voice activation, and enable noise suppression/echo cancellation where supported.  
**FR-MEDIA-02** Every join obtains a short-lived room-scoped LiveKit token **after backend permission checks**. No LiveKit API keys reach clients. One active voice channel per native app; reconnect is automatic within bounded retry.  
**FR-MEDIA-03** Each voice room permits multiple concurrent screen publishers; authorized viewers can pick a stream, focus it, and switch; speaker activity persists independent of selected video. v1 supports one focused stream with optional mini previews, not an arbitrary number of fully decoded 1440p streams.  
**FR-MEDIA-04** Screen or application-window capture, preview, resolution/FPS selection, adaptive bitrate, stop controls, fullscreen, optional system/app audio where platform supports it; unsupported source audio must be made explicit rather than silently promised.  
**FR-MEDIA-05** 720p30 fallback, 1080p60 and 1440p60 supported on certified hardware; hardware encoders/decoders used when tested available, software fallback with lower negotiated quality where necessary. H.264 interoperability baseline; other codecs only after capability negotiation and tested support.  
**FR-MEDIA-06** Camera selection and publish/unpublish in voice rooms; participant video tiles and fullscreen.  
**FR-MEDIA-07** Low-priority background streams are paused or received at reduced layers, avoiding avoidable GPU decode when not visible.  
**Accept:** Two publishers stream different screens in same room; third member switches between them without voice drop. In verified hardware matrix, 1080p60 and 1440p60 render at targeted effective FPS/latency; app indicates degradation transparently. Camera can be enabled simultaneously with voice/screen subject to capture capabilities.

**Security caveat:** LiveKit self-hosted tokens are **not automatically revoked** when participants are removed or permissions updated. Keep join tokens very short-lived (suggested 60 seconds, adjusted after SDK reconnect testing); on ban/kick invoke LiveKit remove participant / update-permissions APIs, stop future backend token issuance, and document the residual reconnect window until old token expiry. Treat “instant permanent ban enforcement” as not guaranteed without additional server controls. Test and monitor this explicitly; short JWT lifetimes alone do not instantly disconnect an already connected participant.

### 4.4 File sharing and storage

**FR-FILE-01** Default per-file cap 25 MiB, server-configurable lower/upper ceilings and quotas; enforce on initiation and streamed bytes received. Authenticated users may upload only to conversations they can access.  
**FR-FILE-02** Storage is a backend abstraction: `put`, `get/range`, `stat`, `delete`. `LocalFilesystemStore` is default; `S3Store` uses AWS SDK for Rust with configurable endpoint, region, bucket, credentials and path style. Backends include RustFS and Cloudflare R2 after conformance testing.  
**FR-FILE-03** Objects are private; backend mediates uploads/downloads in v1, checks authorization before each request and streams data rather than buffering whole files. Use generated, opaque stable keys independent of original filename, storage provider or public URL.  
**FR-FILE-04** Database stores metadata and lifecycle state; API attachment IDs and object keys remain unchanged when backend changes. Validate MIME by sniffing and declared type; sanitize names; prevent inline execution of unsafe types; set safe response headers.  
**FR-FILE-05** Upload intent → byte upload → server validates length/digest and finalizes → attach to permitted message; abandoned partial uploads expire; deletion and retention policies are explicit.  
**FR-FILE-06** Adapter relies on S3 common denominator: `PutObject`, `GetObject` (including range), `HeadObject`, `DeleteObject`. Avoid dependence on public ACLs, bucket notifications, object lock, multipart ETag-as-MD5, vendor-specific metadata or R2-unsupported operations.  
**FR-FILE-07** Backups cover both SQLite and object bytes; migration utility supports object-by-object copy, checksum validation, cutover and rollback before deleting old store.  
**Accept:** upload and download 25 MiB object, reject 25 MiB+1 byte, unauthorized attachment GET denied, migrated RustFS→R2 object still accessible under same app URL and attachment ID after verification; no public bucket configuration required.

**Operational note:** RustFS is optional because it adds a container and baseline resource usage. Docker Compose should expose a `storage-s3-rustfs` profile or equivalent, while the default uses persistent local storage. For private 25 MiB files, app-proxied streaming is a reasonable starting point. Later direct presigned GET/PUT transfers can be added, but URLs are short-lived bearer credentials and browser CORS and S3 endpoint reachability require separate design. Cloudflare R2 currently supports presigned GET/HEAD/PUT/DELETE, not presigned HTML form POST.

### 4.5 Roles and moderation

**FR-MOD-01** Defaults: Owner, Administrator, Moderator, Member. Custom roles allowed; role hierarchy and explicit per-channel Allow/Deny/Neutral overwrites.  
**FR-MOD-02** Granular permission actions (table below) are evaluated on backend; never rely on client hiding controls.  
**FR-MOD-03** User kick, instance/community ban, member timeout, message deletion, revoke invite, force disconnect from voice and moderation audit record.  
**FR-MOD-04** No role assignment/escalation above acting moderator; owner protected from moderation by lower roles; explicit admin transfer can be deferred if server-side recovery is documented.  
**FR-MOD-05** Restore access after timeout, preserve immutable audit records with actor, reason, target, before/after summary and timestamp.  
**Accept:** channel denies override ordinary role allows; users lacking permission cannot upload/send/join/camera/stream by direct API or stale client; moderation events reach online clients; LiveKit token limitation documented and mitigated as above.

## 5. Permission matrix and resolution

**Action bits** (all nullable custom roles use same permission schema): `VIEW_CHANNEL`, `SEND_MESSAGES`, `READ_HISTORY`, `ATTACH_FILES`, `MANAGE_MESSAGES`, `CREATE_INVITES`, `MANAGE_CHANNELS`, `CONNECT_VOICE`, `SPEAK`, `STREAM_SCREEN`, `USE_CAMERA`, `MUTE_MEMBERS`, `MOVE_MEMBERS`, `MANAGE_MEMBERS`, `TIMEOUT_MEMBERS`, `KICK_MEMBERS`, `BAN_MEMBERS`, `MANAGE_ROLES`, `MANAGE_COMMUNITY`, `VIEW_AUDIT_LOG`.

| Permission group / action | Owner | Administrator | Moderator | Member |
|---|:---:|:---:|:---:|:---:|
| View channels / read history | Yes | Yes | Yes | Yes* |
| Send messages / attach files | Yes | Yes | Yes | Yes* |
| Connect / speak in voice | Yes | Yes | Yes | Yes* |
| Share screen / camera | Yes | Yes | Yes | Yes* |
| Create invitations | Yes | Yes | Yes | No (default) |
| Manage other members' messages | Yes | Yes | Yes | No |
| Timeout / kick / ban | Yes | Yes | Yes** | No |
| Mute / move other members | Yes | Yes | Yes** | No |
| Manage channels | Yes | Yes | No (default) | No |
| Manage custom roles | Yes | Yes** | No | No |
| Community configuration | Yes | Yes (non-owner settings) | No | No |
| View audit logs | Yes | Yes | Yes | No |

`*` Subject to channel overwrites and account restrictions. `**` Only on strictly lower role-hierarchy subjects; never the owner. These are **initial templates**, not hardcoded role semantics, except owner authority.

**Resolution order:** Validate authenticated member and account state → owner bypass for community permission checks → collect global role grants → apply `@everyone` channel overwrite → merge role-specific channel overwrites (denies and allows with documented bitwise conflict policy) → apply member-specific overwrite (if supported; deferred implementation allowed only if spec amended) → apply moderation restrictions (ban/timeout) and resource-specific visibility. *Explicit administrative prohibitions (blocked user, ban, object privacy) cannot be overridden by ordinary channel Allow.* Server uses one pure permission evaluator with unit/property tests; resolve any conflicting allow/deny semantics before migrations freeze.

## 6. Data model (SQLite metadata; versioned migrations)

All primary keys are opaque UUID/ULID identifiers. Every SQL table includes `created_at`; soft-deletable entities also include `deleted_at`; relevant rows include `updated_at` and monotonic entity `revision` for optimistic update checks. SQLite foreign keys ON; indexes on access paths; soft deletes plus purge jobs for attachments.

| Entity / table | Essential fields and relationships |
|---|---|
| `instance_settings` (singleton) | `instance_id`, public URL, display name, registration policy, schema/protocol version, attachment limit, media config identifier |
| `communities` | `id`, `name`, `slug`, `owner_user_id`, `is_default`; unique v1 constraint enforced in service until multi-community release |
| `users` | `id`, normalized unique username, display name, avatar attachment, password hash, status, disabled_at |
| `sessions` | `id`, user_id, opaque hashed refresh credential, client/device id, rotated_at, expires_at, revoked_at |
| `invites` | `id`, token_hash, issuer_id, community_id, `role_id?`, max_uses, use_count, expires_at, revoked_at |
| `memberships` | `(community_id,user_id)` unique, joined_at, nickname, timeout_until, ban status (or separate bans) |
| `roles` | `id`, community_id, name, position, color, permission allow bitset, is_system |
| `member_roles` | `(community_id,user_id,role_id)` unique |
| `channel_categories` | `id`, community_id, title, position |
| `channels` | `id`, community_id, category_id?, type text/voice, name, position, topic, archived_at |
| `channel_overwrites` | `(channel_id,target_type,target_id)` unique; allow bitset, deny bitset |
| `dm_threads` | `id`, participant_pair_key unique, created_by, closed_at; v1 strictly exactly 2 users |
| `dm_participants` | `(dm_id,user_id)`, notifications muted, last_read_message_id? |
| `messages` | `id`, author_id, **exactly one** `channel_id` or `dm_id`, text, reply_to_id?, client_nonce, edited_at, deleted_at, creation sequence |
| `reactions` | `(message_id,user_id,emoji)` unique |
| `attachments` | `id`, uploader_id, owner_message_id?, object_key, backend_kind, size_bytes, mime_type, filename, sha256, state pending/ready/deleted, cleanup_deadline |
| `upload_intents` | `id`, user_id, target_channel_or_dm, expected_size, expected_sha256?, expires_at, attachment_id |
| `read_states` | `(user_id,conversation_type,conversation_id)` unique; highest_read_message_seq, last_seen_at |
| `blocks` | `(blocker_user_id,blocked_user_id)` unique, created_at |
| `moderation_actions` | `id`, actor_id, target_user_id?, community_id, action, reason, JSON before_after, created_at; append-only |
| `bans` | `id`, community_id, user_id, moderator_id, reason, revoked_at |
| `durable_events` | **instance-wide** monotonically increasing `seq`, `id`, `type`, audience/subject, payload, created_at, expires_after? |
| `idempotency_keys` | `(actor_id,request_scope,client_nonce)` unique; resulting resource id, expires_at |

**Important constraints/indexes:** `messages` index `(channel_id, created_seq DESC)` and `(dm_id, created_seq DESC)`, plus unique `(author_id, conversation, client_nonce)`; active unrevoked sessions by user; channels by `(community_id,category_id,position)`; read states by user; `durable_events.seq` indexed and never ambiguous across instance; object keys unique; `dm_threads.participant_pair_key` canonicalizes user pair. All authorization queries must scope resources through user membership and instance-owned datastore. No `object_url` persisted as public location.

**Sync retention:** Durable events retained at least 30 days (configurable), or clients use fresh snapshot + channel-level history pagination after cursor expiration. The event cursor is instance-scoped and is not a message timestamp or globally federation-compatible ID.

## 7. HTTP API contract — v1

Base URL `https://{instance}/api/v1`. JSON uses snake_case or camelCase consistently (choose one in OpenAPI; recommended **snake_case** for Rust-first schema). Timestamps RFC3339 UTC; opaque IDs. OpenAPI 3.1 source is authoritative; client/server generated or shared Rust types must remain backward compatible within API major version. JSON errors `{ "error": {"code": "...", "message": "...", "request_id": "...", "details": {}}}`; 401 unauthenticated, 403 unauthorized, 404 inaccessible resources for privacy where appropriate, 409 revision/conflict, 413 oversized, 422 validation, 429 rate limit.

| Endpoint | Use and security |
|---|---|
| `GET /instance` | Instance ID, protocol min/max, default community, capabilities, limits; public sanitized metadata only |
| `POST /auth/register` | Create local user with invitation token (unless operator enabled public registration) |
| `POST /auth/login`, `/auth/refresh`, `/auth/logout`, `/auth/logout-all` | Issue/rotate/revoke sessions; refresh token stored hashed server-side |
| `GET /me`, `PATCH /me` | Profile and settings |
| `GET /community`, `PATCH /community` | Default community info / permitted admin changes |
| `GET /channels`, `POST /channels`, `PATCH /channels/{id}`, `DELETE /channels/{id}` | List/edit channels with permission checks |
| `GET /channels/{id}/messages?before=...&limit=...` | Bounded pagination (max 100) |
| `POST /channels/{id}/messages`, `PATCH /messages/{id}`, `DELETE /messages/{id}` | Idempotent message creation with `client_nonce`, revision checks on edits |
| `PUT /messages/{id}/reactions/{emoji}`, `DELETE ...` | Reactions |
| `GET /dms`, `POST /dms`, `GET /dms/{id}/messages`, `POST /dms/{id}/messages` | One-to-one DMs; authorize both participants |
| `GET /members`, `PATCH /members/{id}`, `POST /members/{id}/timeout` | Members and timeouts |
| `GET /roles`, `POST /roles`, `PATCH /roles/{id}`, `DELETE /roles/{id}` | Roles; elevation/hierarchy safe |
| `PUT /channels/{id}/overwrites/{target}` | Role/member channel overwrite |
| `POST /invites`, `GET /invites`, `DELETE /invites/{id}` | Invite management |
| `POST /moderation/kick`, `/moderation/ban`, `/moderation/unban` | Audited moderation actions |
| `GET /audit?cursor=...` | Permission-restricted audit history |
| `POST /attachments/intents` | Validate conversation + quota; returns attachment ID + upload path |
| `PUT /attachments/{id}/content`, `POST /attachments/{id}/complete` | Stream bytes, verify size/hash, finalize |
| `GET /attachments/{id}/content`, `DELETE /attachments/{id}` | Permission-checked download/deletion, Range optional |
| `POST /media/join` | Validate CONNECT_VOICE, issue short LiveKit token scoped to selected voice room |
| `POST /media/leave` | Client cleanup and optional server-side state reconciliation |
| `GET /sync?after={cursor}&limit=...` | Durable authorized event backfill or `cursor_expired` snapshot recovery instruction |
| `GET /health/live`, `/health/ready` | Health; ready checks metadata and media dependencies without exposing secrets |

**Security requirements:** Never accept a client-chosen S3 object key or LiveKit room ID without mapping/authorizing it against an internal object/channel. Attachment completion requires byte receipt; message creation cannot attach another user's pending upload. Return only access-authorized event payloads. Rate limit auth, invites, message sends, upload starts and media joins.

## 8. Realtime gateway and event contract

WebSocket: `wss://{instance}/api/v1/gateway`; authenticate using an authorized session mechanism that does **not** leak long-lived tokens through URLs or logs. One authenticated connection per instance/client profile. The gateway is for control and messaging events; media uses LiveKit directly.

**Wire envelope examples** (illustrative, final naming must match OpenAPI/AsyncAPI):

```json
{
  "op": "event",
  "v": 1,
  "event_id": "01J...",
  "cursor": "97231",
  "type": "message.created",
  "occurred_at": "2026-10-24T00:00:00Z",
  "data": {
    "message_id": "01J...",
    "channel_id": "01J...",
    "author_id": "01J...",
    "revision": 1
  }
}
```

Client command request includes `request_id` and for mutating idempotent operations `client_nonce`; server replies `ack` or typed `error` with matching request ID. **No exactly-once network claim**: at-least-once replay, monotonic per-instance `cursor`, idempotent writes, duplicate suppression, and gap/backfill give exactly-once *visible application effects*.

**Durable event catalog:** `message.created|updated|deleted`, `reaction.added|removed`, `channel.created|updated|deleted`, `role.updated|deleted`, `member.updated|timed_out|banned`, `invite.revoked`, `attachment.ready|deleted`, `dm.created`, `permission.changed`, `moderation.action`. **Ephemeral catalog:** `presence.changed`, `typing.started|stopped`, `voice.participants_changed`, `stream.started|stopped`, `camera.started|stopped` (media state may originate from LiveKit events mirrored to authorized clients). Ephemeral events are not guaranteed replayed; clients reconstruct current state from authorized snapshot.

**Reconnect contract:** exponential backoff + jitter; send last committed cursor, replay missing authorized events or request snapshot if expired; no cross-instance cursor reuse; detect sequence gaps; when access revoked, remove affected cached content where required and stop subscriptions. A client may background-throttle non-visible instances, but badge updates should continue when socket is healthy. Durable events must be filtered per user *at delivery and backfill* (permission changes can alter audience).

## 9. Desktop application architecture and flows

### Client modules

- `desktop-gpui`: native shell, unified instance rail, channels/messages, calls/stream UI, accessibility and keyboard shortcuts.
- `client-core`: isolated profiles, service orchestration, session lifecycle, connection registry, optimistic UI, retry/backoff.
- `client-cache`: SQLite namespace keyed by `instance_id + account_id`, message cache, read cursors, upload drafts.
- `protocol`: typed request/response/domain data and versioned gateway envelopes.
- `audio-engine`: capture/output, device switches, PTT, VAD, echo/noise processing and meter.
- `media-engine`: LiveKit signaling/transport, capture, encoder negotiation, published/subscribed tracks, adaptive quality.
- `gpui-video`: GPU-aware decoded-frame bridge and render surface; independent feasibility gate.

**Unified rail flow:** Click Add Instance → input HTTPS address → fetch `/instance` and show identity → authenticate/register using invite → cache `instance_id + account_id` → rail icon appears → automatically synchronize channels and badges. Switching rail icons changes visible workspace, not login state. Context menu: reconnect, preferences, sign out, remove local profile. The sidebar must never present a merged DM inbox implying cross-instance messaging.

**Admin first-run flow:** Compose up → open initial administrator setup page/API (scoped one-time token) → provision community and channels → create invite → user registers/logs in → messages → voice → stream.

**Voice & stream flow:** Join voice → validate backend permissions → get short-lived LiveKit join token → audio connect → choose screen/window/camera → capability negotiation → publish one or more tracks per policy → multiple users may publish concurrently → viewer selects stream and quality → stop → cleanup publisher track and state.

**Platform caveat:** GPUI has Windows/macOS/Linux platform backends, but native screen source pickers, system-audio capture, tray/global PTT and efficient GPU textures require OS-specific integration. No UI thread may block on capture/codec/network; handle rendering/capture threading explicitly. Linux Wayland permissions use portal/PipeWire semantics; X11 and other OS backends require tested paths. Release must document unsupported combinations if any; critical required capabilities need platform certification.

## 10. Docker/Compose deployment requirements

**Default, one-node, Linux host:** `api` (Axum), `livekit` (SFU), `caddy` (HTTPS reverse proxy); persistent SQLite and local files in named or bind-mounted volumes. Use stable tags/digests, non-root service users, secrets/env example, TLS, readiness/liveness checks, migration job/entrypoint, logs and restart policies. Expose necessary WebRTC UDP/TURN ports explicitly; a plain HTTP tunnel cannot substitute for UDP media connectivity. **Redis** is optional for initial single-node LiveKit; document production recommendation and make it available as profile. Distributed LiveKit uses Redis and is out of scope. `rustfs` is an opt-in Compose storage profile; no internet cloud storage prerequisite. Production server target is Linux even though desktop target includes Windows and macOS.

**Operator requirements:** `docker compose up -d` with a documented secrets bootstrap sequence, `docker compose ps` healthy, upgrade with DB backup + migrations, `backup` / `restore` documented with verified restore tests, storage migration utility, privacy-preserving logs, volume ownership, no mandatory vendor telemetry, explicit external domains and open ports. Instance first-launch checks flag invalid public domain, storage unwritable, incompatible LiveKit credentials and unreachable media endpoints.

**Storage example configuration (illustrative; names may evolve):**

```dotenv
STORAGE_DRIVER=filesystem  # or s3
LOCAL_STORAGE_PATH=/var/lib/app/objects
# Used when STORAGE_DRIVER=s3
S3_ENDPOINT=http://rustfs:9000   # or https://<account>.r2.cloudflarestorage.com
S3_REGION=us-east-1             # R2 uses auto
S3_BUCKET=private-attachments
S3_ACCESS_KEY_ID=...
S3_SECRET_ACCESS_KEY=...
S3_FORCE_PATH_STYLE=true        # provider-dependent
ATTACHMENT_MAX_BYTES=26214400
```

**Migration procedure:** Freeze writes or enable controlled dual-write only if explicitly implemented → consistent metadata backup → copy each attachment by opaque object key to target storage → verify count, per-object size and stored SHA-256 (don't equate S3 ETag to SHA-256) → switch config → read smoke-test attachments and restored messages → retain old objects through defined rollback window → delete only after confirmation. Never hardcode provider URLs in database records. Document object retention for removed messages and attachments.

## 11. Security, abuse and privacy

- TLS 1.2+ and authenticated WebSocket; DTLS/SRTP via WebRTC. No general E2EE promise.
- Strong local passwords hashed with Argon2id; rotating revocable refresh session credentials, tokens excluded from logs; OS keychain or secure OS credential interface for desktop.
- Access controls validated at REST, gateway subscription/backfill, file download, upload completion and LiveKit join/publish grants; permissions changes remove stale subscriptions.
- Invite tokens stored hashed; expiry/max uses checked transactionally; no privilege escalation via invite-selected role.
- File size quotas, MIME/content sniffing, `Content-Disposition: attachment` for dangerous types, sanitized Markdown, image decode limits, path traversal and zip bomb defenses; symlinks disallowed for local object path operations.
- Operator may access data at rest. Make server trust model visible; no default recording; user-facing camera/screen indicators and quick stop.
- Admin audit events append-only; admins cannot rely on encrypted transport for E2EE; backup secrets are separately protected.
- Abuse protections: login/invite/media join rates, upload limits, spam throttles and bounded WebSocket message sizes; no public storage buckets needed.
- Threat-model the self-hosted LiveKit stale-token reconnection window and test kick/ban denial through auth server; never promise instantaneous revocation beyond actual capability.

## 12. Non-functional goals and validation gates

| Area | v1 target / test gate | Method |
|---|---|---|
| Native idle efficiency | Aspirational <250 MiB RSS and <1% CPU steady idle on defined reference desktop, not universal guarantee | Measure 10-min idle, no stream, one and three instances; document variance |
| Chat UI | 60 FPS target when scrolling 10k-message history with virtualization; no full-history load | Frame-time profiling on reference OS/hardware |
| Messaging delivery | P95 under 300 ms in same-region lab for online text messages (app-level create → peer visible) | 1k-message test, controlled network |
| Sync correctness | No duplicate visible messages or missing durable events after offline interval or restart | Fault-injection tests, simulated disconnect/reorder |
| Voice | Push-to-talk, device switching, noise processing, reliable reconnect; no UI hangs | Hardware/audio reference matrix, packet loss tests |
| Streaming | 1080p60 and 1440p60 supported on certified encoders/decoders, two simultaneous streamers selectable | 10-min test per OS/codec/GPU; capture effective FPS, dropped frames, GPU load, end-to-end latency |
| Video GPU integration | No unbounded decoded frame queue; bound latency and copies where feasible | Decoder-renderer tracing and texture upload profiling |
| Access control | 100% negative permission tests for all protected endpoints/events/media join | Unit/integration/property tests, stale session attempts |
| Storage portability | Local ↔ RustFS ↔ R2 conformance and migration fixtures | API tests for put/get/range/head/delete, copy/verify/read |
| Docker operational readiness | Fresh install, restart, upgrade migration, backup and restore tested | CI Compose smoke suite and manual deployment QA |
| Client platform coverage | Windows/macOS/Linux packages; capture and audio device matrix published | CI builds plus manual integration rigs |

**Go/no-go:** M0 must prove GPUI decoded video rendering + capture + LiveKit quality on at least one target OS. Full release cannot assert 1440p60 or feature parity without measured matrix results. Hardware/camera/system-audio limitation states must be visible, not silently bypassed.

## 13. Milestones and engineering deliverables

| Milestone | Scope | Completion / exit gate |
|---|---|---|
| **M0 — Feasibility spike** | GPUI shell + 2 Rust LiveKit clients + microphone audio + native screen capture + decoded GPU view; benchmark 1080p60/1440p60 | Document measured hardware/OS support, codec path, copies, failures, media topology decision |
| **M1 — Protocol & infrastructure** | Cargo workspace; OpenAPI/AsyncAPI drafts, migrations, Axum server; Compose, TLS, health, SQLite; CI | Fresh Docker deployment; schema/API contracts linted; bootstrapped owner |
| **M2 — Identity & multi-instance** | Invite accounts, sessions/refresh, secure keychain, profile registry, unified rail, connection isolation | 3 distinct instances/accounts operate side-by-side with isolated caches |
| **M3 — Messaging and files** | Channels/categories, replies/reactions, DMs, read/unread, event cursor/backfill, 25 MiB upload, FS/S3 adapter | Offline sync and duplication tests; local/RustFS/R2 storage contract tests |
| **M4 — Voice** | LiveKit rooms/tokens, device controls, PTT, VAD, echo/noise management, voice presence | Two+ users talk, reconnect, mute; unauthorized rejoin mitigated and tested |
| **M5 — Screenshare & webcam** | Multi-publisher stream UI, screen/window capture, webcam, system audio capability, hardware codec policies | Two streams simultaneously, third viewer picks/switches; verified 1080p60/1440p60 matrix |
| **M6 — Roles/moderation** | Custom roles, overwrites, audit, kicks/bans/timeouts, permission enforcement | Permission matrix tests at REST/WS/LiveKit/file layers, no obvious escalation |
| **M7 — Release hardening** | Packages/updaters, accessibility, diagnostics, backups, ops docs, migration, load/security testing | Windows/Linux/macOS smoke tests; documented install/restore; release checklist signed off |

**Implementation ordering:** Build media technical spike early, before committing to GPUI-specific video rendering architecture. The final user-facing MVP includes every selected feature, but interim engineering milestones are not public v1 releases. Early milestones can be delivered as preview builds.

## 14. Test plan and release acceptance checklist

### Mandatory end-to-end scenarios

1. Clean Docker setup initializes exactly one default community and owner; second community creation rejected in v1.
2. Invitation expires, is single-use when configured so, and cannot elevate a new user above issuer permission; open registration stays disabled by default.
3. Windows/Linux/macOS client can authenticate to instance A and B (and optional C), all shown in one unified rail, with distinct unread counters, credentials and client caches.
4. User A posts/edits/deletes text in allowed channel; user B receives typed events; reconnect/backfill produces same state without duplicates.
5. DMs cannot be retrieved by third user even with known IDs; blocked user policy is enforced.
6. Attachment 25 MiB succeeds; oversized fails; deleted/hidden-channel attachment cannot be fetched using old URL; malicious filenames/paths cannot escape storage root.
7. Voice join grants only the authorized room and publish capabilities; a banned user cannot obtain fresh token. Explicitly record expected stale-token TTL limit in self-hosted LiveKit.
8. Two simultaneous screen streams in one voice channel remain available; viewer focuses and switches them, controls individual audio and exits without affecting other streams.
9. 1080p60 and 1440p60 certified hardware streams pass measured frame rate, drop, decode/render and audio-sync acceptance thresholds; unsupported GPU receives explainable fallback.
10. Webcam shares alongside voice and screen on supported sources; permissions denied produce clear errors.
11. Moderator cannot moderate higher-ranked role or assign prohibited privileges; denies and timeouts take effect on text/file/media permission gates.
12. Operator backs up, resets deployment, restores and recovers messages, user accounts, uploads, roles and audit logs.
13. RustFS→Cloudflare R2 migration preserves attachment IDs/URLs to clients, correctly validates checksums and permissions, and supports rollback.

### Definition of done for each milestone

Updated contract/schema migrations; automated unit/integration tests; relevant end-to-end manual tests; permission abuse cases; performance regression measurements where appropriate; operator/user documentation; CI across supported targets; dependency/license inventory; changelog/ADR for behavior changes.

## 15. Risks and mitigations

| Risk | Impact | Mitigation / owner |
|---|---|---|
| GPUI pre-1.0 breaking APIs | Frequent desktop refactors | Pin tested version/revision, isolate UI layer, track upstream |
| Cross-platform 1440p60 hardware pipeline | Quality/fps parity uneven | M0 hardware matrix, platform-specific capture/codec backends, fallbacks |
| GPUI decoded video integration | UI jank/CPU frame copies | Dedicated `gpui-video` proof-of-concept and frame budget tracking |
| LiveKit self-hosted access tokens not immediately revocable | Kicked user potentially rejoins until expiry | Very short grants; immediate participant removal; backend denial; explicit threat model |
| CGNAT/UDP/TURN | Media unavailable for some users | Setup network validator, operator guides, optional relay/VPS |
| Storage provider differences | Broken uploads/migration | Portable subset contract and provider conformance suite |
| Extra container overhead | Small VPS resource pressure | Local FS default, optional RustFS, SQLite metadata, disable unnecessary services |
| Password recovery without email | User lockout | Document administrator-driven account reset; no centralized email dependency |
| SQLite single-node writes/load | Throughput bottleneck at scale | WAL, measured indexes, bounded worker writes, migration path to PostgreSQL later |
| Overly broad v1 scope | Slower delivery | Strict milestones, early feasibility, scope change control |

## 16. Open ADRs (not blockers to initial M0–M2)

- Final open-source license after checking crates, bundled native codec binaries and distribution requirements (explicitly **undecided**).
- Exact desktop packaging/updater approach for each OS.
- Exact username/identifier and account-recovery policy (per-instance admin reset versus optional email).
- Reaction/mention syntax, emoji catalog, markdown policy and retention period variations.
- Whether custom member-specific channel overrides ship in v1 or only role-level overrides, and definitive multi-role allow/deny merge ordering; must resolve before M6.
- Whether client previews more than one stream simultaneously; fixed expectation is multiple publishers and selectable focused stream.
- Reference GPU hardware test matrix and quantified streaming latency/drop budgets finalized after M0.
- File quota defaults and admin storage retention settings beyond fixed 25 MiB default max file size.
- Caddy public endpoint naming and TURN networking template appropriate to different host providers.

## 17. External implementation references (accessed 2026-10-24)

- GPUI platform/readme and pre-1.0 caveat: https://github.com/zed-industries/zed/blob/main/crates/gpui/README.md
- LiveKit token behavior, including self-hosted revocation limitation: https://docs.livekit.io/frontends/reference/tokens-grants/
- LiveKit single-node deployment and Redis recommendation: https://docs.livekit.io/transport/self-hosting/deployment/
- LiveKit distributed Redis requirement: https://docs.livekit.io/transport/self-hosting/distributed/
- RustFS 1.0 GA announcement: https://rustfs.com/blog/announcing-rustfs-1-0-0-ga/
- RustFS container and minimum test resources: https://docs.rustfs.com/en/installation/container and https://docs.rustfs.com/en/installation/requirement/checklists/hardware-selection
- Cloudflare R2 S3 compatibility and presigned URL support: https://developers.cloudflare.com/r2/api/s3/api/ and https://developers.cloudflare.com/r2/api/s3/presigned-urls/

---

**Status:** Baseline approved for engineering planning based on current decisions; decisions expressly marked *assumption* or *open ADR* remain open for technical validation. Freeze API/permission edge cases only after contract tests and M0 media feasibility results are reviewed.
