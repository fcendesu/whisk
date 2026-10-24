#!/bin/sh
# Dedicated loopback SFU tests. Tokens are short-lived and never printed.
set -eu
WHISK_PROJECT_ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$WHISK_PROJECT_ROOT"
docker compose -f spikes/media/fixture/compose.yaml up -d
curl -fsS --retry 5 --retry-connrefused --retry-delay 1 http://127.0.0.1:7880 >/dev/null
# Unique, isolated project so concurrent test jobs cannot tear each other down.
WHISK_FAILURE_PROJECT="whisk-media-blocked-$$"
trap 'docker compose -p "$WHISK_FAILURE_PROJECT" -f spikes/media/fixture/compose-unreachable.yaml down >/dev/null 2>&1' EXIT
docker compose -p "$WHISK_FAILURE_PROJECT" -f spikes/media/fixture/compose-unreachable.yaml up -d
WHISK_FAILURE_ADDRESS=$(docker compose -p "$WHISK_FAILURE_PROJECT" -f spikes/media/fixture/compose-unreachable.yaml port livekit 7880)
if [ -z "$WHISK_FAILURE_ADDRESS" ]; then echo 'The isolated fixture has no published signaling port.' >&2; exit 1; fi
curl -fsS --retry 5 --retry-connrefused --retry-delay 1 "http://$WHISK_FAILURE_ADDRESS" >/dev/null
LIVEKIT_UNREACHABLE_MEDIA_URL="ws://$WHISK_FAILURE_ADDRESS"
export LIVEKIT_UNREACHABLE_MEDIA_URL
LIVEKIT_URL=ws://127.0.0.1:7880
LIVEKIT_PUBLISH_TOKEN=$(python3 spikes/media/fixture/token.py --publisher)
LIVEKIT_TOKEN=$(python3 spikes/media/fixture/token.py)
LIVEKIT_SECOND_PUBLISH_TOKEN=$(python3 spikes/media/fixture/token.py --publisher)
LIVEKIT_NOISE_TOKEN=$(python3 spikes/media/fixture/token.py --publisher)
export LIVEKIT_URL LIVEKIT_PUBLISH_TOKEN LIVEKIT_SECOND_PUBLISH_TOKEN LIVEKIT_TOKEN LIVEKIT_NOISE_TOKEN
cargo test --workspace --all-features --locked -- --ignored --nocapture
