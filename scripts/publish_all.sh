#!/usr/bin/env sh
set -e
export RUST_BACKTRACE=full

# Publish the proc-macro crate first, then the library that depends on it.
cargo publish -p maybe-once-macros
cargo publish -p maybe-once
