#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
python3 scripts/prepare-offline.py
node scripts/schema.cjs
mkdir -p test-output
node scripts/v1-oracle.cjs test-output/oracle.json
node -e 'const fs=require("fs"),a=JSON.parse(fs.readFileSync("test-output/oracle.json")),b=JSON.parse(fs.readFileSync("fixtures/v1-oracle.json")); delete a.node;delete b.node;require("assert/strict").deepEqual(a,b);console.log("Executable immutable V1/useForm oracle reproduced")'
node scripts/renderer.cjs
python3 scripts/registration-test.py
cargo fmt --check
cargo fmt --manifest-path tests/beta-harness/Cargo.toml --check
cargo fmt --manifest-path tests/beta-harness/runtime/Cargo.toml --check
for toolchain in stable 1.85.0; do
  cargo "+$toolchain" test --locked --offline
  cargo "+$toolchain" test --manifest-path tests/beta-harness/Cargo.toml --locked --offline
 done
cargo clippy --all-targets --locked --offline -- -D warnings
cargo clippy --manifest-path tests/beta-harness/Cargo.toml --all-targets --locked --offline -- -D warnings
cargo clippy --manifest-path tests/beta-harness/runtime/Cargo.toml --lib --locked --offline -- -D warnings
for manifest in Cargo.toml tests/beta-harness/Cargo.toml; do
 cargo check --manifest-path "$manifest" --all-targets --locked --offline --target x86_64-pc-windows-gnu
 cargo clippy --manifest-path "$manifest" --all-targets --locked --offline --target x86_64-pc-windows-gnu -- -D warnings
 done
cargo clippy --manifest-path tests/beta-harness/runtime/Cargo.toml --lib --locked --offline --target x86_64-pc-windows-gnu -- -D warnings
