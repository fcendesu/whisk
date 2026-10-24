#!/bin/sh
# Dedicated loopback SFU tests. Tokens are short-lived and never printed.
set -eu
WHISK_PROJECT_ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$WHISK_PROJECT_ROOT"
docker compose -f spikes/media/fixture/compose.yaml up -d
curl -fsS --retry 5 --retry-connrefused --retry-delay 1 http://127.0.0.1:7880 >/dev/null
LIVEKIT_URL=ws://127.0.0.1:7880
LIVEKIT_PUBLISH_TOKEN=$(python3 spikes/media/fixture/token.py --publisher)
LIVEKIT_TOKEN=$(python3 spikes/media/fixture/token.py)
LIVEKIT_SECOND_PUBLISH_TOKEN=$(python3 spikes/media/fixture/token.py --publisher)
LIVEKIT_NOISE_TOKEN=$(python3 spikes/media/fixture/token.py --publisher)
export LIVEKIT_URL LIVEKIT_PUBLISH_TOKEN LIVEKIT_SECOND_PUBLISH_TOKEN LIVEKIT_TOKEN LIVEKIT_NOISE_TOKEN
cargo test --workspace --all-features --locked -- --ignored --nocapture
