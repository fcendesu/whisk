# Whisk — ADR-0003: Decisions requiring validation or approval

**Status:** Open (not user-locked)

| Area | Proposed default | Decide by |
|---|---|---|
| License | Remain undecided pending Rust/C++ codec/LiveKit dependency and redistribution audit | Before public release |
| Permission overwrite merging | Role-specific allows win over conflicting role denies; explicit member overwrite last; bans always override | Before M6 schema |
| Multi-role direct member overrides | Include v1 if tests/DDL remain manageable | Before M6 |
| Gateway cursor/watermark schema | Instance-wide event sequence, explicit watermark; filtered gaps are allowed | M1 protocol freeze |
| Authentication recovery | Document owner-driven reset; no mandatory email dependency | M2 |
| GPU platform matrix | Certify specific Windows/Linux/macOS capture + encode/decode paths and fallbacks | M0 + M5 |
| Native auto-update | Platform signing and installer/update strategy undecided | M7 |
| Single-node LiveKit Redis | Avoid mandatory Redis unless pinned LiveKit release/deployment mode requires it | M0/M1 |
| File quotas/retention | 25 MiB max per file fixed, aggregate quotas configurable with documented default later | M3 |
| Advanced media previews | Multiple publishers, one focused viewer stream with optional lightweight previews | M5 |

Unresolved entries must not silently become user decisions. Propose, test, document, and obtain review at milestone gate.
