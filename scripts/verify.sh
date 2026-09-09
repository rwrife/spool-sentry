#!/bin/sh
set -eu

ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
PROOF="$ROOT/firmware/toolchain-proof"
DOMAIN="$ROOT/firmware/domain"

if ! command -v cargo >/dev/null 2>&1 && [ -f "$HOME/.cargo/env" ]; then
    # rustup's user-scoped installer documents this environment file.
    . "$HOME/.cargo/env"
fi

python3 "$ROOT/scripts/check_docs.py"

HOST=$(rustc -vV | sed -n 's/^host: //p')
[ -n "$HOST" ]
echo "host target: $HOST"

echo "== firmware/toolchain-proof =="
cd "$PROOF"
cargo fmt --all -- --check
cargo clippy --lib --target "$HOST" --locked -- -D warnings
cargo test --lib --target "$HOST" --locked
cargo clippy --release --locked -- -D warnings
cargo build --release --locked

PROOF_ELF="$PROOF/target/riscv32imc-unknown-none-elf/release/spool-sentry-toolchain-proof"
test -s "$PROOF_ELF"
BYTES=$(wc -c < "$PROOF_ELF" | tr -d ' ')
echo "proof ESP32-C3 linked ELF: PASS ($BYTES bytes)"

echo "== firmware/domain =="
cd "$DOMAIN"
cargo fmt --all -- --check
# Host tests: allocation-free domain core with injected adapter fakes.
cargo clippy --lib --tests --target "$HOST" --locked -- -D warnings
cargo test --target "$HOST" --locked
# ESP32-C3 target: no_std clippy, release link, size report.
cargo clippy --release --locked -- -D warnings
cargo build --release --locked

DOMAIN_ELF="$DOMAIN/target/riscv32imc-unknown-none-elf/release/spool-sentry-domain"
test -s "$DOMAIN_ELF"
BYTES=$(wc -c < "$DOMAIN_ELF" | tr -d ' ')
echo "domain ESP32-C3 linked ELF: PASS ($BYTES bytes)"
if command -v riscv32-esp-elf-size >/dev/null 2>&1; then
    riscv32-esp-elf-size -A "$DOMAIN_ELF"
elif command -v size >/dev/null 2>&1; then
    size -A "$DOMAIN_ELF" || true
fi
echo "VERIFY: PASS"
