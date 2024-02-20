#!/usr/bin/env bash
set -euo pipefail
IFS=$'\n\t'

pushd frontend
trunk build --release
popd

pushd server-embedded
cargo build --release
popd
elf2uf2-rs --deploy --serial --verbose target/thumbv6m-none-eabi/release/server-embedded

