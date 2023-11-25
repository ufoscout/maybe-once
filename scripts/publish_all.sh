#!/usr/bin/env sh
set -e
export RUST_BACKTRACE=full

cargo publish 
