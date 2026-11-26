#!/usr/bin/env bash
set -euo pipefail

# Multi-Container & Multi-OS Test Harness for ng-native-engine
# Supports Docker or Podman across:
#   - ubuntu   : Ubuntu 24.04 LTS (GTK 4.14 + Libadwaita + JSC 6.0 + Node 24 + Vitest coverage + E2E)
#   - debian12 : Debian 12 Bookworm (GTK 4.8 CSS fallback + QuickJS-NG + Node 24 + E2E)
#   - fedora   : Fedora 41 (GTK 4.16+ + Libadwaita + WebKitGTK 6.0 + Node 24 + E2E)
#   - windows  : Windows x86_64 cross-compile (MinGW-w64) + Wine64 execution + Node 24
#   - macos    : macOS (aarch64-apple-darwin & x86_64-apple-darwin) strict Clippy verification
#   - linux    : Runs ubuntu + debian12 + fedora
#   - all      : Runs all 5 container suites sequentially
# Optional second arg: --e2e-only to run strict E2E harness inside the container.

TARGET="${1:-all}"
MODE="${2:-}"

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

  echo "==> [${RUNTIME}] Running suite in ${tag}..."
  if [[ "${MODE}" == "--e2e-only" && "${label}" != "macos-cross" ]]; then
    if [[ "${label}" == "windows-wine64" ]]; then
      "${RUNTIME}" run --rm "${tag}" sh -c "pnpm install --frozen-lockfile && pnpm run build && NG_E2E_REQUIRE_FULL=1 xvfb-run -a cargo test --target x86_64-pc-windows-gnu --test test_e2e_harness -- --nocapture"
    else
      "${RUNTIME}" run --rm "${tag}" sh -c "pnpm install --frozen-lockfile && pnpm run build && NG_E2E_REQUIRE_FULL=1 xvfb-run -a cargo test --test test_e2e_harness -- --nocapture"
    fi
  else
    "${RUNTIME}" run --rm "${tag}"
  fi
}

case "${TARGET}" in
  ubuntu)
    run_container_suite "ubuntu-24.04" "docker/Dockerfile.ubuntu-gtk4"
    ;;
  debian12)
    run_container_suite "debian-12" "docker/Dockerfile.debian12-gtk4"
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
  linux)
    run_container_suite "ubuntu-24.04" "docker/Dockerfile.ubuntu-gtk4"
    run_container_suite "debian-12" "docker/Dockerfile.debian12-gtk4"
    run_container_suite "fedora-41" "docker/Dockerfile.fedora-gtk4"
    ;;
  all)
    run_container_suite "ubuntu-24.04" "docker/Dockerfile.ubuntu-gtk4"
    run_container_suite "debian-12" "docker/Dockerfile.debian12-gtk4"
    run_container_suite "fedora-41" "docker/Dockerfile.fedora-gtk4"
    run_container_suite "windows-wine64" "docker/Dockerfile.windows-wine"
    run_container_suite "macos-cross" "docker/Dockerfile.macos-cross"
    ;;
  *)
    echo "Usage: $0 [ubuntu|debian12|fedora|windows|macos|linux|all] [--e2e-only]" >&2
    exit 1
    ;;
esac
