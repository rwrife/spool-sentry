#!/bin/sh
set -eu

ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
PROOF="$ROOT/firmware/toolchain-proof"

if ! command -v cargo >/dev/null 2>&1 && [ -f "$HOME/.cargo/env" ]; then
    # rustup's user-scoped installer documents this environment file.
    . "$HOME/.cargo/env"
fi

python3 "$ROOT/scripts/check_docs.py"
cd "$PROOF"
cargo fmt --all -- --check

HOST=$(rustc -vV | sed -n 's/^host: //p')
[ -n "$HOST" ]
echo "host target: $HOST"

cargo clippy --lib --target "$HOST" --locked -- -D warnings
cargo test --lib --target "$HOST" --locked
cargo clippy --release --locked -- -D warnings
cargo build --release --locked

ARTIFACT="$PROOF/target/riscv32imc-unknown-none-elf/release/spool-sentry-toolchain-proof"
test -s "$ARTIFACT"
BYTES=$(wc -c < "$ARTIFACT" | tr -d ' ')
echo "ESP32-C3 linked ELF: PASS ($BYTES bytes)"
echo "VERIFY: PASS"
