# Whisk — Documentation index

Whisk is the user-selected working name for this self-hosted communication app. See [`adr/0004-whisk-brand-identity.md`](adr/0004-whisk-brand-identity.md) for naming rationale and release checks.

## Authoritative product baseline

- [`prd.md`](prd.md): full PRD, locked user decisions, scope, acceptance and risks.
- [`architecture.md`](architecture.md): instances, packages, storage/media boundaries and deployment topology.
- [`database.md`](database.md): planned SQLite tables, keys, indexes, transaction invariants.
- [`api.md`](api.md): planned REST API behavior and error/security contract.
- [`events.md`](events.md): durable WS events and reconnect rules.
- [`permissions.md`](permissions.md): roles, bitflags, overwrites, moderation and API security.
- [`deployment.md`](deployment.md): Compose topology, media connectivity, storage backends, backup/migration.
- [`testing.md`](testing.md): multi-platform acceptance, risks and quality gates.

## Superpowers workflow

- [`superpowers/README.md`](superpowers/README.md): execution order and design/plan relationship.
- [`superpowers/specs/`](superpowers/specs/): product plus subsystem design docs (why, boundaries, tradeoffs).
- [`superpowers/plans/`](superpowers/plans/): M0–M7 implementation plans (what files, tests and commands).
- [`adr/`](adr/): decisions, assumptions and unresolved choices.

**Precedence:** explicit user-locked PRD decisions > approved ADRs > subsystem specs > candidate interfaces in implementation plans. In a conflict, flag it for review rather than silently redefining scope.
