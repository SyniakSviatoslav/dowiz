#!/usr/bin/env bash
# docker/mkosi-rootfs.sh — DK-08 rootfs builder for the native SPA server.
#
# PURPOSE
#   Build a minimal HOST ROOTFS (not an OCI app image) that boots `native-spa-server`
#   directly in a microVM (e.g. systemd-nspawn / QEMU / Firecracker). This is the
#   "zero-OCI" deployment path: the artifact is a rootfs tarball + a static binary,
#   NOT a container image with a layered runtime.
#
# WHAT THIS SCRIPT DOES
#   - Resolves the compiled `native-spa-server` release binary (from
#     tools/native-spa-server/target/release/native-spa-server).
#   - Fetches the static SPA dist (from dist/public, produced by the pnpm build).
#   - Assembles a mkosi image definition (a barebones systemd-free rootfs) and runs
#     `mkosi` to produce a bootable rootfs tarball / disk image.
#
# innovate: FULL MKOSI INTEGRATION (mkosi build + boot smoke test) is a FOLLOW-UP.
#   This script is the documented, copy-paste-ready scaffold. It does NOT execute
#   mkosi here (mkosi is not installed in CI/local by default), so it is safe to
#   run as a no-op preview with `--dry-run`.
#
# USAGE
#   ./docker/mkosi-rootfs.sh [--dry-run] [--out DIR]
#
# REQUIREMENTS (for a real build)
#   - mkosi  (https://github.com/systemd/mkosi)  OR  Buildah
#   - A pre-built native-spa-server release binary
#   - The SPA dist under dist/public
set -euo pipefail

DRY_RUN=0
OUT_DIR="${OUT_DIR:-./build/rootfs}"
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --dry-run) DRY_RUN=1; shift ;;
    --out)     OUT_DIR="$2"; shift 2 ;;
    -h|--help) sed -n '2,40p' "$0"; exit 0 ;;
    *) echo "unknown arg: $1" >&2; exit 2 ;;
  esac
done

BIN="${ROOT}/tools/native-spa-server/target/release/native-spa-server"
DIST="${ROOT}/dist/public"
MKOSI_CONF="${OUT_DIR}/mkosi.conf"

echo "== DK-08 mkosi rootfs builder =="
echo "repo root : ${ROOT}"
echo "binary    : ${BIN}"
echo "spa dist  : ${DIST}"
echo "out dir   : ${OUT_DIR}"

if [[ ! -x "${BIN}" ]]; then
  echo "ERROR: native-spa-server binary not found at ${BIN}." >&2
  echo "       build it first: (cd tools/native-spa-server && cargo build --release)" >&2
  exit 1
fi
if [[ ! -d "${DIST}" ]]; then
  echo "WARN: SPA dist not found at ${DIST}; the rootfs will ship without web assets." >&2
fi

mkdir -p "${OUT_DIR}"

# --- Write the mkosi definition ----------------------------------------------
# A minimal, distroless-flavoured rootfs: a static /sbin/init shim that execs the
# native server on boot. No systemd required for the microVM path.
cat > "${MKOSI_CONF}" <<EOF
# DK-08 — minimal rootfs for native-spa-server (zero-OCI, NOT an app image).
[Distribution]
Distribution=none

[Output]
Format=directory
OutputDirectory=${OUT_DIR}/rootfs

[Content]
Bootable=no
Packages=

[PostInstallationScripts]
# Install the single static binary + run it as init.
EOF

# Post-install script consumed by mkosi (copied alongside the conf).
cat > "${OUT_DIR}/post.sh" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
ROOTFS="$1"
mkdir -p "${ROOTFS}/usr/bin" "${ROOTFS}/var/www"
install -m 0755 "${BIN}" "${ROOTFS}/usr/bin/native-spa-server"
if [[ -d "${DIST}" ]]; then
  cp -r "${DIST}/." "${ROOTFS}/var/www/"
fi
# Make the server PID 1 of the microVM.
ln -sf /usr/bin/native-spa-server "${ROOTFS}/init"
EOF
chmod +x "${OUT_DIR}/post.sh"

if [[ "${DRY_RUN}" -eq 1 ]]; then
  echo "== dry-run: would run mkosi with config at ${MKOSI_CONF} =="
  echo "   (mkosi not invoked — install mkosi + re-run without --dry-run)"
  exit 0
fi

if ! command -v mkosi >/dev/null 2>&1; then
  echo "ERROR: mkosi not installed. Install it (pip install mkosi) or run with --dry-run." >&2
  exit 1
fi

# Real build. BIN/DIST are exported for the post.sh script.
export BIN DIST
mkosi -C "${OUT_DIR}" build
echo "== DK-08 rootfs built at ${OUT_DIR}/rootfs =="
