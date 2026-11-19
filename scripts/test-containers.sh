#!/usr/bin/env bash
set -euo pipefail

# Multi-Container OS Test Harness for ng-native-engine
# Supports Docker or Podman to verify dynamic FFI (GTK4, Libadwaita, JavaScriptCore, libsoup3, SQLite3, inotify)
# inside isolated Ubuntu 24.04 and Fedora 41 containers with Xvfb virtual display servers.

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
  local distro="$1"
  local dockerfile="$2"
  local tag="ng-native-test:${distro}"

  echo "==> [${RUNTIME}] Building ${tag} from ${dockerfile}..."
  "${RUNTIME}" build -f "${dockerfile}" -t "${tag}" .

  echo "==> [${RUNTIME}] Running Xvfb + GTK4 + FFI test suite in ${tag}..."
  "${RUNTIME}" run --rm "${tag}"
}

case "${TARGET}" in
  ubuntu)
    run_container_suite "ubuntu-24.04" "docker/Dockerfile.ubuntu-gtk4"
    ;;
  fedora)
    run_container_suite "fedora-41" "docker/Dockerfile.fedora-gtk4"
    ;;
  all)
    run_container_suite "ubuntu-24.04" "docker/Dockerfile.ubuntu-gtk4"
    run_container_suite "fedora-41" "docker/Dockerfile.fedora-gtk4"
    ;;
  *)
    echo "Usage: $0 [ubuntu|fedora|all]" >&2
    exit 1
    ;;
esac
