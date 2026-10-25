# Development workflow

Track work in [GitHub Issues](https://github.com/fcendesu/whisk/issues/2) so scope,
acceptance and evidence are visible before and after implementation.

1. Pick a ready issue and verify its prerequisites. Before starting a later
   milestone, split its planned tasks into linked issues with descriptions,
   scope, acceptance criteria, dependencies and local verification steps.
   Reconcile proposed plan paths/interfaces against the actual workspace.
2. Mark the selected issue `status:in-progress` and add a short implementation
   note. Keep one active implementation issue at a time. Develop directly in
   the primary repository checkout on `main`; use a worktree only if the owner
   explicitly requests one.
3. Implement the issue's scope. Use a meaningful failing test before new
   behavior, then implement and verify the fix. Documentation-only changes do
   not need behavior tests. Keep the product invariants in `docs/AGENTS.md`.
4. Run relevant local checks and record actual results. Device observations
   and generated-media tests prove different things; preserve that distinction.
   Obtain explicit user action for physical recording tests and preserve
   tested permissioned app bundles. CI stays manual: do not enable automatic
   triggers or dispatch a run unless the owner asks.
5. Make small Conventional Commits that reference the issue, for example
   `fix(media): diagnose relay connection failures` with `Refs #12` in the body.
   Push the authorized work and post its commit links, verification results,
   and remaining limitations on the issue.
6. Check acceptance criteria only when evidence supports them. If a required
   check or user action is missing, keep the issue open and describe the
   blocker with `status:blocked`. A passing retry does not prove an intermittent
   failure fixed. Use `Closes #N` or close the issue only when all criteria pass.
   Remove the active status label, update its parent checklist, and move to
   the next ready issue. Complete a milestone only after its acceptance review.

Status labels are mutually exclusive: `status:ready`, `status:in-progress`, or
`status:blocked`. Closed issues need no active status label. `tracking` marks
roadmap/milestone parents; `evidence` marks completed, supported baseline work.
Bug and enhancement labels describe the kind of work, independently of status.

See [the tracker index](ISSUES.md) for milestone links and the initial work queue.
