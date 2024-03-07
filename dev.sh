#!/usr/bin/env bash
set -euo pipefail
IFS=$'\n\t'

(trap 'kill 0' SIGINT; \
 bash -c 'cd frontend; trunk serve' & \
 bash -c 'cd server-test; cargo watch -- cargo run --bin server-test -- --port 8081')
