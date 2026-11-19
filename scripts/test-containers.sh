#!/usr/bin/env bash
set -euo pipefail

# Multi-Container & Multi-OS Test Harness for ng-native-engine
# Supports Docker or Podman across:
#   - ubuntu  : Ubuntu 24.04 LTS (GTK4 + Libadwaita + JavaScriptCore 6.0 + libsoup3 + SQLite3 + inotify + Xvfb)
#   - fedora  : Fedora 41 (GTK4 + Libadwaita + WebKitGTK 6.0 + libsoup3 + SQLite3 + Xvfb)
#   - windows : Windows x86_64 cross-compile (MinGW-w64) + Wine64 execution of Windows .exe test binaries
#   - macos   : macOS (aarch64-apple-darwin & x86_64-apple-darwin) strict Clippy & target verification
#   - all     : Runs all 4 container suites sequentially

TARGET="${1:-all}"

if command -v podman >/dev/null 2>&1; then
  RUNTIME="podman"
elif command -v docker >/dev/null 2>&1; then
  RUNTIME="docker"
else
  echo "[Error] Neither 'podman' nor 'docker' is installed." >&2
  exit 1
fi

run_container_suite() {
  local label="$1"
  local dockerfile="$2"
  local tag="ng-native-test:${label}"

  echo "==> [${RUNTIME}] Building ${tag} from ${dockerfile}..."
  "${RUNTIME}" build -f "${dockerfile}" -t "${tag}" .

  echo "==> [${RUNTIME}] Running test suite in ${tag}..."
  "${RUNTIME}" run --rm "${tag}"
}

case "${TARGET}" in
  ubuntu)
    run_container_suite "ubuntu-24.04" "docker/Dockerfile.ubuntu-gtk4"
    ;;
  fedora)
    run_container_suite "fedora-41" "docker/Dockerfile.fedora-gtk4"
    ;;
  windows)
    run_container_suite "windows-wine64" "docker/Dockerfile.windows-wine"
    ;;
  macos)
    run_container_suite "macos-cross" "docker/Dockerfile.macos-cross"
    ;;
  all)
    run_container_suite "ubuntu-24.04" "docker/Dockerfile.ubuntu-gtk4"
    run_container_suite "fedora-41" "docker/Dockerfile.fedora-gtk4"
    run_container_suite "windows-wine64" "docker/Dockerfile.windows-wine"
    run_container_suite "macos-cross" "docker/Dockerfile.macos-cross"
    ;;
  *)
    echo "Usage: $0 [ubuntu|fedora|windows|macos|all]" >&2
    exit 1
    ;;
esac
