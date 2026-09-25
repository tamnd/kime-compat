#!/usr/bin/env bash
# Starts a kime binary on the laya checkpoint and runs every live surface against it: the response
# contract on the committed requests, then the TypeSafe JS SDK examples under each JavaScript
# runtime given. Both this repository's CI and tamnd/kime's run this, so they check the same things.
#
#   ci/against-kime.sh <kime binary> <models dir> [runtime ...]
#
# A runtime is a command that runs sdk-js/run.mjs, such as node or bun. With KIME_COMPAT_SDK_PYTHON
# set to a python that has typesafe-sdk 0.7.1, the committed requests also go through the TypeSafe
# Python SDK. The server log is printed if anything fails.
set -euo pipefail

bin="$1"
models="$2"
shift 2
here="$(cd "$(dirname "$0")/.." && pwd)"
port="${KIME_COMPAT_PORT:-18800}"
url="http://127.0.0.1:$port"
log="$(mktemp)"

"$bin" serve --port "$port" --models "$models/laya" --device cpu --precision f32 > "$log" 2>&1 &
server=$!
trap 'kill "$server" 2> /dev/null || true' EXIT
fail() {
  echo "$1"
  echo "server log:"
  cat "$log"
  exit 1
}

for _ in $(seq 1 120); do
  if curl --silent --fail "$url/v1/models" > /dev/null; then break; fi
  kill -0 "$server" 2> /dev/null || fail "kime serve exited before it listened"
  sleep 1
done
curl --silent --fail "$url/v1/models" > /dev/null || fail "kime serve did not listen within 120 seconds"

(cd "$here" && cargo run --quiet --release -- live "$url") || fail "the response contract failed"

(cd "$here/sdk-js" && npm ci --no-audit --no-fund --silent)
for runtime in "$@"; do
  (cd "$here/sdk-js" && KIME_COMPAT_URL="$url" "$runtime" run.mjs) || fail "the JS SDK examples failed on $runtime"
done

if [ -n "${KIME_COMPAT_SDK_PYTHON:-}" ]; then
  "$KIME_COMPAT_SDK_PYTHON" "$here/sdk-python/run.py" "$url" || fail "the Python SDK replay failed"
fi
