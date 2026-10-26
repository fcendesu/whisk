# Whisk development tracker

GitHub Issues is the live source of work status. The product docs define requirements;
the dated milestone plans provide the implementation starting point. Their historical
checkboxes are not proof that a feature is complete.

- [Follow the v1 roadmap](https://github.com/fcendesu/whisk/issues/2)
- [Browse milestones](https://github.com/fcendesu/whisk/milestones)
- [Ready work](https://github.com/fcendesu/whisk/issues?q=is%3Aissue+is%3Aopen+label%3A%22status%3Aready%22)
- [Development workflow](CONTRIBUTING.md)

## Milestones

| Milestone | Tracker | Plan |
|---|---|---|
| [M0 — Native media feasibility](https://github.com/fcendesu/whisk/milestone/1) | [#3](https://github.com/fcendesu/whisk/issues/3) | [Source plan](docs/docs/superpowers/plans/2026-10-24-m0-media-feasibility.md) |
| [M1 — Server foundation](https://github.com/fcendesu/whisk/milestone/2) | [#4](https://github.com/fcendesu/whisk/issues/4) | [Source plan](docs/docs/superpowers/plans/2026-10-24-m1-foundation.md) |
| [M2 — Identity and native client](https://github.com/fcendesu/whisk/milestone/3) | [#5](https://github.com/fcendesu/whisk/issues/5) | [Source plan](docs/docs/superpowers/plans/2026-10-24-m2-identity-client.md) |
| [M3 — Messaging and private storage](https://github.com/fcendesu/whisk/milestone/4) | [#6](https://github.com/fcendesu/whisk/issues/6) | [Source plan](docs/docs/superpowers/plans/2026-10-24-m3-messaging-storage.md) |
| [M4 — Voice](https://github.com/fcendesu/whisk/milestone/5) | [#7](https://github.com/fcendesu/whisk/issues/7) | [Source plan](docs/docs/superpowers/plans/2026-10-24-m4-voice.md) |
| [M5 — Streaming and webcam](https://github.com/fcendesu/whisk/milestone/6) | [#8](https://github.com/fcendesu/whisk/issues/8) | [Source plan](docs/docs/superpowers/plans/2026-10-24-m5-streaming-webcam.md) |
| [M6 — Permissions and moderation](https://github.com/fcendesu/whisk/milestone/7) | [#9](https://github.com/fcendesu/whisk/issues/9) | [Source plan](docs/docs/superpowers/plans/2026-10-24-m6-permissions-moderation.md) |
| [M7 — Release hardening](https://github.com/fcendesu/whisk/milestone/8) | [#10](https://github.com/fcendesu/whisk/issues/10) | [Source plan](docs/docs/superpowers/plans/2026-10-24-m7-release-hardening.md) |

## Initial work queue

Snapshot: 2026-10-26. Use GitHub for current status. M0 stays open; M1 is blocked
on the M0 acceptance decision. Later milestones will gain detailed task issues before
their implementation starts.

| Work | Issue | Initial status |
|---|---|---|
| M0: Record completed native media baseline | [#11](https://github.com/fcendesu/whisk/issues/11) | Completed evidence |
| M0: Diagnose intermittent local TURN relay connection timeout | [#12](https://github.com/fcendesu/whisk/issues/12) | Ready |
| M0: Verify physical microphone continuity across video focus switching | [#13](https://github.com/fcendesu/whisk/issues/13) | Ready |
| M0: Measure sustained real capture through the GPUI viewer | [#14](https://github.com/fcendesu/whisk/issues/14) | Ready |
| M0: Verify permission and device recovery without unsafe signing changes | [#15](https://github.com/fcendesu/whisk/issues/15) | Ready |
| M0: Record M0 acceptance and the M1 go/no-go decision | [#16](https://github.com/fcendesu/whisk/issues/16) | Blocked |
| M1: Cargo workspace and protocol IDs | [#17](https://github.com/fcendesu/whisk/issues/17) | Blocked |
| M1: SQLite pool and migrations | [#18](https://github.com/fcendesu/whisk/issues/18) | Blocked |
| M1: One-time owner bootstrap | [#19](https://github.com/fcendesu/whisk/issues/19) | Blocked |
| M1: Instance discovery and health | [#20](https://github.com/fcendesu/whisk/issues/20) | Blocked |
| M1: Durable event outbox and gateway foundation | [#21](https://github.com/fcendesu/whisk/issues/21) | Blocked |
| M1: Compose deploy smoke | [#22](https://github.com/fcendesu/whisk/issues/22) | Blocked |

The relay fixture fix and its evidence are tracked in [#12](https://github.com/fcendesu/whisk/issues/12).
A separately observed high-profile generated-audio gap is tracked in [#23](https://github.com/fcendesu/whisk/issues/23).
The completed baseline records existing work; it does not certify GPU presentation,
high-profile real capture, platform recovery, or release quality.
