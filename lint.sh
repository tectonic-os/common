#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")"

if [ "${1:-}" = "--fix" ]; then
    cargo fmt
    echo "lint: formatted the source"
    exit
fi

cargo fmt --check || {
    echo "lint: unformatted, run ./lint.sh --fix" >&2
    exit 1
}
echo "lint: the source is clean"

cargo deny --locked check bans licenses sources
echo "lint: the dependency policy accepts the locked graph"

# Only the scheduled fuzz job builds the fuzz targets, so this check keeps
# their entry points compiling on every push.
cargo check --locked --features fuzzing
echo "lint: the fuzz entry points compile"

cargo clippy --locked --all-targets -- -D warnings
echo "lint: clippy accepts every target"

# Both consumers build common with test-support, so its public surface is
# checked with the feature on as well as off.
cargo clippy --locked --all-targets --features test-support -- -D warnings
echo "lint: clippy accepts the test-support surface"

cargo nextest run --locked
echo "lint: the components do what they did"
