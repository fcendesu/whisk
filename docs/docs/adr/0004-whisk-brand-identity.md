# Whisk — ADR-0004: Working product name and brand direction

**Decision date:** 2026-10-24  
**Status:** Accepted working name; public brand clearance pending  
**Scope:** Product naming and documentation, not new v1 functionality

## Context

The project owner chose **Whisk** for the native, self-hosted communication app inspired by Discord and MicUp, but not positioned as a full Discord replacement. The name was inspired by the playful habit of using a **kitchen whisk or wooden spatula as a makeshift microphone** when talking with friends. The app focuses on voice, chat, video and game/screen streaming for small groups.

## Decision

- Use **Whisk** as the working product name across all planning materials.
- Use **Whisk Desktop** for the GPUI client, **Whisk Server** for the Docker-ready Axum service, and **Whisk Media** for the LiveKit integration. The names identify parts of one product; they do not imply separate public commercial offerings.
- **Whisk CLI** is a name reserved for a possible future administration tool, **not** an additional v1 deliverable.
- In v1, one Whisk Server *instance* hosts one community. One Whisk Desktop client may connect to multiple independent instances in a unified sidebar. Branding does not merge data or accounts between instances.
- No chosen brand asset is required before implementation. A kitchen-whisk-as-microphone motif and the tagline *“Your space. Your voice.”* are optional **proposals**, not confirmed final creative.

## Alternatives considered

- **Voxlet:** more overtly voice-oriented, but not selected.
- Generic descriptive name: not selected; less memorable and doesn't capture the inspiration.

## Consequences

- Update PRD, project docs, plans and agent instructions to refer to **Whisk**.
- Keep implementation paths and external protocols stable unless independently changed through design review. Do not add branding to identifiers that must remain machine-stable.
- Before publishing under the name, verify trademark conflicts, domain availability, repository/namespace availability, and package/bundle identifiers; document the outcomes.
- Project license is still undecided and unaffected by the name choice.

## Validation gate

- Product-facing text and release checklists use Whisk consistently.
- No references remain to an **unnamed** product in active specs.
- Naming clearance is explicitly tracked as pending until investigated.
- No logo, assets, registrations, public release or actual binaries are implied by this ADR.
