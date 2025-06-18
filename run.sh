#!/bin/bash

set -em

pids=()

cleanup() {
  echo "--- shutting down processes ---"
  for pid in "${pids[@]}"; do
    kill -s sigint -- "-$pid" 2>/dev/null
  done
  wait
}

trap cleanup exit

usage() {
  echo "usage: $0 [install|start|preview]"
  exit 1
}

install_deps() {
  echo "--- installing frontend dependencies ---"
  (cd frontend && bun install)
  echo "--- building backend dependencies (release) ---"
  (cd backend && cargo build --release)
}

deps_installed() {
  [[ -d frontend/node_modules ]] && [[ -f backend/target/release/$(basename "$(pwd)") ]]
}

ensure_deps() {
  if ! deps_installed; then
    echo "--- dependencies missing, running install ---"
    install_deps
  fi
}

start_dev() {
  ensure_deps
  echo "--- starting backend (api) [dev] ---"
  (cd backend && cargo run) &
  pids+=($!)

  echo "--- starting frontend (web) [dev] ---"
  (cd frontend && bun run dev) &
  pids+=($!)

  echo "--- dev services started. pids: ${pids[*]} ---"
  echo "--- press ctrl-c to shut down ---"

  wait -n
}

start_preview() {
  ensure_deps
  echo "--- building frontend for production ---"
  (cd frontend && bun run build)

  echo "--- starting backend (api) [release] ---"
  (cd backend && cargo run --release) &
  pids+=($!)

  echo "--- starting frontend (web) [preview] ---"
  (cd frontend && bun run preview) &
  pids+=($!)

  echo "--- preview services started. pids: ${pids[*]} ---"
  echo "--- press ctrl-c to shut down ---"

  wait -n
}

command=${1:-start}

case "$command" in
  install)
    install_deps
    ;;
  start)
    start_dev
    ;;
  preview) 
    start_preview
    ;;
  *)
    usage
    ;;
esac
