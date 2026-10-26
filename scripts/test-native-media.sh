#!/bin/sh
# Dedicated loopback SFU tests. Tokens are short-lived and never printed.
set -eu
WHISK_PROJECT_ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$WHISK_PROJECT_ROOT"
WHISK_DIAG_RUN=""
WHISK_RAW_TEST_LOG=""
if [ -n "${WHISK_MEDIA_DIAGNOSTIC_DIR:-}" ]; then
  WHISK_DIAG_RUN="$WHISK_MEDIA_DIAGNOSTIC_DIR/run-$$"
  mkdir -p "$WHISK_MEDIA_DIAGNOSTIC_DIR"
  mkdir "$WHISK_DIAG_RUN"
fi
docker compose -f spikes/media/fixture/compose.yaml up -d
curl -fsS --retry 5 --retry-connrefused --retry-delay 1 http://127.0.0.1:7880 >/dev/null
# Unique, isolated project so concurrent test jobs cannot tear each other down.
WHISK_FAILURE_PROJECT="whisk-media-blocked-$$"
WHISK_RELAY_PROJECT="whisk-media-relay-$$"
cleanup() {
  if [ -n "$WHISK_DIAG_RUN" ]; then
    docker compose -p "$WHISK_RELAY_PROJECT" -f spikes/media/fixture/compose-relay.yaml logs --no-log-prefix 2>/dev/null |
      python3 scripts/media_diagnostics.py server > "$WHISK_DIAG_RUN/relay-server.jsonl" ||
      printf '%s\n' 'Unable to collect sanitized relay diagnostics.' >&2
  fi
  if [ -n "$WHISK_RAW_TEST_LOG" ]; then
    rm -f "$WHISK_RAW_TEST_LOG" || printf '%s\n' 'Unable to remove temporary test output.' >&2
  fi
  docker compose -p "$WHISK_FAILURE_PROJECT" -f spikes/media/fixture/compose-unreachable.yaml down >/dev/null 2>&1 || :
  docker compose -p "$WHISK_RELAY_PROJECT" -f spikes/media/fixture/compose-relay.yaml down >/dev/null 2>&1 || :
}
trap cleanup EXIT
docker compose -p "$WHISK_FAILURE_PROJECT" -f spikes/media/fixture/compose-unreachable.yaml up -d
WHISK_FAILURE_ADDRESS=$(docker compose -p "$WHISK_FAILURE_PROJECT" -f spikes/media/fixture/compose-unreachable.yaml port livekit 7880)
if [ -z "$WHISK_FAILURE_ADDRESS" ]; then echo 'The isolated fixture has no published signaling port.' >&2; exit 1; fi
curl -fsS --retry 5 --retry-connrefused --retry-delay 1 "http://$WHISK_FAILURE_ADDRESS" >/dev/null
LIVEKIT_UNREACHABLE_MEDIA_URL="ws://$WHISK_FAILURE_ADDRESS"
export LIVEKIT_UNREACHABLE_MEDIA_URL
docker compose -p "$WHISK_RELAY_PROJECT" -f spikes/media/fixture/compose-relay.yaml up -d
WHISK_RELAY_ADDRESS=$(docker compose -p "$WHISK_RELAY_PROJECT" -f spikes/media/fixture/compose-relay.yaml port livekit 7880)
if [ -z "$WHISK_RELAY_ADDRESS" ]; then echo 'The relay fixture has no published signaling port.' >&2; exit 1; fi
curl -fsS --retry 5 --retry-connrefused --retry-delay 1 "http://$WHISK_RELAY_ADDRESS" >/dev/null
LIVEKIT_RELAY_URL="ws://$WHISK_RELAY_ADDRESS"
export LIVEKIT_RELAY_URL
LIVEKIT_URL=ws://127.0.0.1:7880
LIVEKIT_PUBLISH_TOKEN=$(python3 spikes/media/fixture/token.py --publisher)
LIVEKIT_TOKEN=$(python3 spikes/media/fixture/token.py)
LIVEKIT_SECOND_PUBLISH_TOKEN=$(python3 spikes/media/fixture/token.py --publisher)
LIVEKIT_NOISE_TOKEN=$(python3 spikes/media/fixture/token.py --publisher)
export LIVEKIT_URL LIVEKIT_PUBLISH_TOKEN LIVEKIT_SECOND_PUBLISH_TOKEN LIVEKIT_TOKEN LIVEKIT_NOISE_TOKEN
if [ -z "$WHISK_DIAG_RUN" ]; then
  cargo test --workspace --all-features --locked -- --ignored --nocapture
else
  WHISK_RAW_TEST_LOG=$(mktemp "${TMPDIR:-/tmp}/whisk-native-test.XXXXXX")
  WHISK_TEST_STATUS=0
  cargo test --workspace --all-features --locked -- --ignored --nocapture > "$WHISK_RAW_TEST_LOG" 2>&1 || WHISK_TEST_STATUS=$?
  cat "$WHISK_RAW_TEST_LOG" || printf '%s\n' 'Unable to replay test output.' >&2
  python3 scripts/media_diagnostics.py client < "$WHISK_RAW_TEST_LOG" > "$WHISK_DIAG_RUN/client.jsonl" ||
    printf '%s\n' 'Unable to collect sanitized client diagnostics.' >&2
  printf '{"event":"runner_exit","exit_code":%s}\n' "$WHISK_TEST_STATUS" >> "$WHISK_DIAG_RUN/client.jsonl" ||
    printf '%s\n' 'Unable to save test exit diagnostics.' >&2
  exit "$WHISK_TEST_STATUS"
fi
