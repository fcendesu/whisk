# Whisk — WebSocket + sync contract

**Endpoint:** `wss://{instance}/api/v1/gateway`. Authentication follows the instance session; a gateway handshake includes last durable cursor plus protocol version. All cursor values belong to exactly one instance.

## Envelope

```json
{
  "v": 1,
  "event_id": "opaque-event-id",
  "seq": 456,
  "type": "message.created",
  "occurred_at": "2026-10-24T12:00:00Z",
  "data": { "message_id": "opaque-msg-id", "channel_id": "opaque-channel-id", "revision": 1 }
}
```

`seq` is present only for durable events and monotonically advances at the instance level. Ephemeral events have no durable sequence and are not replayed. Clients acknowledge highest **fully applied** sequence, never an event received but not persisted to local cache.

**Durable event names:** `message.created`, `message.updated`, `message.deleted`, `reaction.added`, `reaction.removed`, `channel.created`, `channel.updated`, `channel.deleted`, `role.updated`, `role.deleted`, `member.updated`, `member.timed_out`, `member.banned`, `permission.changed`, `attachment.ready`, `attachment.deleted`, `dm.created`, `invite.revoked`, `moderation.action`.

**Ephemeral event names:** `typing.started`, `typing.stopped`, `presence.changed`, `voice.participants_changed`, `stream.started`, `stream.stopped`, `camera.started`, `camera.stopped`.

## Replay contract

1. Server stores durable event in DB transaction with the resource mutation, then sends authorized events over WS.
2. Client persists message/read model and new cursor atomically in its `(instance_id, account_id)` cache.
3. After disconnect, WS reconnect or `/sync?after=...` returns authorized relevant events and an instance watermark.
4. Events may arrive at least once. Client de-duplicates `event_id` and entity revision. It never relies on exactly-once network delivery.
5. If cursor is older than retention (30 days default), server returns `cursor_expired`, and client obtains fresh authorized snapshot plus paginated history.
6. Revoked channel access invalidates local subscriptions and any affected cached content; backfill checks visibility at replay time.
7. Sequence numbers **may have gaps for a specific user** due to filtered private events. Do not infer dropped events from every numeric gap; use committed cursor/watermark and explicit sync completeness protocol.

**Open decision:** define exact gateway handshake/ack JSON schema and whether `/sync` returns page watermark separately from highest emitted event. M1 contract tests must freeze this before client-core implementation.
