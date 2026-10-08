#!/usr/bin/env bash
# Run from a logged-in macOS desktop session. Synthetic notes only.
set -euo pipefail
cd "$(dirname "$0")/.."
output="${1:-$PWD/docs/performance/latest.json}"
mkdir -p "$(dirname "$output")"
export CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_DEV_OPT_LEVEL=3 CARGO_INCREMENTAL=0
build_log="$(mktemp /tmp/sparkpad-benchmark-build.XXXXXX)"
trap 'rm -f "$build_log"' EXIT
cargo test --locked --test rich_text --no-run --message-format=json > "$build_log"
benchmark_binary="$(python3 - "$build_log" <<'PYTHON'
import json, sys
for line in open(sys.argv[1]):
    try:
        item = json.loads(line)
    except ValueError:
        continue
    if item.get("reason") == "compiler-artifact" and item.get("target", {}).get("name") == "rich_text" and item.get("executable"):
        print(item["executable"])
PYTHON
)"
if [[ -z "$benchmark_binary" ]]; then
    echo "Native performance executable was not found." >&2
    exit 1
fi
SPARKPAD_PERF_OUTPUT="$output" /usr/bin/time -l "$benchmark_binary"
