#!/usr/bin/env sh
set -e
export RUST_BACKTRACE=full

cargo test --workspace
cargo test --workspace --all-features
