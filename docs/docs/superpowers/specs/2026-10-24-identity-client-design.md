# Whisk — Identity and Native Multi-Instance UX — Design

**Scope:** invitations, local sessions, desktop profiles and caches, navigation. References: `../../prd.md` §4.1 and §9, `../../architecture.md`.

## User experience

A new user clicks **Add instance** in a unified Discord-like rail, enters an HTTPS base URL, checks displayed instance identity/version, registers through an invitation or signs in, and adds that instance to the left rail. Switching rail icons changes visible channels and DMs, without switching processes or global identities. Unread and notifications remain instance-specific.

## Model

Each instance has independent users/roles/sessions, session refresh rotation, revoked-credential support and hash-stored invitations with expiry/max uses. Client profiles include `instance_id`, `account_id`, base URL, encrypted OS credential reference and cache namespace. Multiple WS connections may exist; one active voice channel globally.

## Security and failure modes

No implicit trust on HTTPS hostname change or instance ID change. Never put bearer/refresh token in SQLite cache/logs. Expired invite and token rotation replay deny registration/refresh; one instance down must not block other UI. Removing a profile optionally purges only its local cache, leaving remote account untouched.

## Alternatives

Central sign-in would break independent self-hosting; one local identity across instances would misrepresent trust boundaries. Native keychain use is preferred over encryption with bundled application key.
