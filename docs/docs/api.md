# Whisk — HTTP API contract — candidate v1 endpoints

**Base:** `https://{instance}/api/v1`. JSON fields are `snake_case`; timestamps RFC3339 UTC; IDs opaque. This document is contract guidance; M1 will create and validate an OpenAPI 3.1 source file `contracts/openapi.yaml`.

| Verb | Path | Capability / authorization |
|---|---|---|
| GET | `/instance` | Public sanitized metadata, stable instance ID, API/version and feature range |
| POST | `/auth/register` | Valid invite unless operator enabled public signup |
| POST | `/auth/login`, `/auth/refresh`, `/auth/logout`, `/auth/logout-all` | Password/session lifecycle and rotating refresh |
| GET/PATCH | `/me` | Own profile only |
| GET/PATCH | `/community` | Read membership and authorized configuration change |
| GET/POST | `/channels` | List allowed channels / `MANAGE_CHANNELS` |
| PATCH/DELETE | `/channels/{id}` | `MANAGE_CHANNELS` and revision |
| GET/POST | `/channels/{id}/messages` | `READ_HISTORY` / `SEND_MESSAGES` |
| PATCH/DELETE | `/messages/{id}` | Author or `MANAGE_MESSAGES` per policy |
| PUT/DELETE | `/messages/{id}/reactions/{emoji}` | Authenticated, channel-visible |
| GET/POST | `/dms` | Instance-local 1:1 participants; blocked-user restrictions |
| GET/POST | `/dms/{id}/messages` | Participant-only |
| GET/POST/PATCH/DELETE | `/roles`, `/roles/{id}` | Role hierarchy and `MANAGE_ROLES` |
| PUT | `/channels/{id}/overwrites/{target}` | Authorize permission-overwrite edits |
| GET/POST/DELETE | `/invites`, `/invites/{id}` | `CREATE_INVITES` and issuer restrictions |
| POST | `/moderation/kick`, `/moderation/ban`, `/moderation/unban` | `KICK_MEMBERS`, `BAN_MEMBERS`; append audit |
| POST | `/members/{id}/timeout` | `TIMEOUT_MEMBERS` |
| GET | `/audit` | `VIEW_AUDIT_LOG` |
| POST | `/attachments/intents` | `ATTACH_FILES`, quota, allowed target |
| PUT | `/attachments/{id}/content` | Only initiator, enforce stream length <= configured max |
| POST | `/attachments/{id}/complete` | Validate size/hash and transition `ready` |
| GET | `/attachments/{id}/content` | Revalidate resource visibility, private bytes, optional Range |
| POST | `/media/join` | `CONNECT_VOICE`, room-scoped LiveKit grant |
| GET | `/sync?after={cursor}&limit={n}` | Reauthorized durable event backfill |
| GET | `/health/live`, `/health/ready` | Operations health without leaking secrets |

**Message create sample:**

```json
{ "text": "gg", "client_nonce": "018f3af7-8c4b-7d11-813f-e35f62a5d001", "reply_to_id": null, "attachment_ids": [] }
```

**Error envelope:**

```json
{ "error": { "code": "forbidden", "message": "Action is not permitted", "request_id": "req_...", "details": {} } }
```

Use `400` malformed request, `401` unauthenticated, `403` forbidden, `404` inaccessible/unknown resource as policy requires, `409` revision/idempotency conflict, `413` oversized, `422` field validation, `429` throttled. Errors must never leak whether private DM/resource IDs exist.

## Security contract

- All protected operations authorize **on each request**, not only at login. Refresh tokens stored only as hashes server-side; never emit LiveKit API secret or S3 credentials.
- `POST /media/join` returns a token for only one authorized room with restricted publish rights, short TTL; separate screen/camera grants as applicable.
- `GET /instance` permits compatibility checks before sign-in but must not reveal admin secrets, user list or private channel structure.
- Upload bytes stream to temporary private storage; `complete` validates digest, size and state. Content GET never redirects to a public unauthenticated bucket.
- JSON endpoints have bounded body sizes and paginated results; `GET /channels/{id}/messages` default 50/max 100.

## Contract evolution

Every endpoint has contract tests from `contracts/openapi.yaml`; breaking changes require `/api/v2` or negotiated protocol version. Media availability is a capability field, not a reason text API becomes unavailable.
