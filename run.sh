#!/bin/bash

set -em

# determine the js package manager. prefer bun, fallback to npm.
if command -v bun &>/dev/null; then
  pkg_mgr="bun"
else
  pkg_mgr="npm"
fi
echo "--- using '$pkg_mgr' for js tasks ---"

pids=()

cleanup() {
  echo "--- shutting down processes ---"
  for pid in "${pids[@]}"; do
    # kill the whole process group. this is correct.
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
  (cd frontend && "$pkg_mgr" install)
  echo "--- building backend dependencies (release) ---"
  (cd backend && cargo build --release)
}

# this check is kinda naive but fine for this purpose.
deps_installed() {
  local backend_name
  backend_name=$(basename "$(pwd)")
  [[ -d frontend/node_modules ]] &&
    [[ -f backend/target/release/$backend_name ]]
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
  # npm needs 'run' explicitly for custom scripts. bun doesn't, but this works for both.
  (cd frontend && "$pkg_mgr" run dev) &
  pids+=($!)

  echo "--- dev services started. pids: ${pids[*]} ---"
  echo "--- press ctrl-c to shut down ---"

  wait -n
}

start_preview() {
  ensure_deps
  echo "--- building frontend for production ---"
  (cd frontend && "$pkg_mgr" run build)

  echo "--- starting backend (api) [release] ---"
  (cd backend && cargo run --release) &
  pids+=($!)

  echo "--- starting frontend (web) [preview] ---"
  (cd frontend && "$pkg_mgr" run preview) &
  pids+=($!)

  echo "--- preview services started. pids: ${pids[*]} ---"
  echo "--- press ctrl-c to shut down ---"

  wait -n
}

command=${1:-start}

case "$command" in
install) install_deps ;;
start) start_dev ;;
preview) start_preview ;;
*) usage ;;
esac
