# Whisk — ADR-0001: Local backend instance, unified native client

**Status:** Accepted in product discussion (2026-10-24)

**Context:** Friends self-host independent communication servers, but a user should access all of them through one native GPUI desktop application.

**Decision:** Each backend deployment supports exactly one community in v1 while storing community IDs for later expansion; the native client has a Discord-like sidebar listing many independently authenticated instances. Accounts, credentials, cache, read state, WebSocket cursors, files and roles never cross instance boundaries. One voice channel active per desktop client.

**Alternatives rejected for v1:** Single backend account with global identity (central dependence), federation (scope explosion), separate app windows per instance (poor UX).

**Consequences:** Instance-qualified IDs and caches, parallel WS connections with backpressure limits, non-merged DMs, resilient failure isolation. Tests connect three servers with distinct credentials and simultaneous unread markers.
