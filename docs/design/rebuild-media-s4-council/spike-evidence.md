# S4 — Q3 spike evidence (image-stack feasibility, run in the ACTUAL build env)

> Lane: isolated worktree `agent-ad562cc71d9c3d9b1`, projects under `rebuild/spikes/image-stack/`
> (`attempt-a-libvips/`, `attempt-b-pure-rust/`). This answers the packet's Q3 with machine evidence;
> the question row stays in `open-questions.md` for the record, disposition lands in the RESOLVE.

## Verdict: pure-Rust `image` 0.25 + `webp` 0.3 (libwebp-sys)

- **Builds everywhere identically** — dev sandbox, CI, Docker. `libwebp-sys` compiles vendored libwebp
  via the `cc` crate only: **zero cmake anywhere in the dependency graph** (verified via cargo tree +
  build.rs grep). No system packages required at all.
- **Lossy quality parity confirmed**: the `image` crate's native WebP writer is LOSSLESS-only (433KB vs
  13KB on the identical frame — no quality knob), so the `webp` crate is REQUIRED for q78/80/82 — and it
  built and ran here cleanly.
- **Performance** (release, best-of-5, synthetic non-degenerate 2000×1500 PNG): product profile
  (800×800 q82) **76ms** total (decode 14 / resize 42 / encode 20); logo (512×512 q80) **64ms**;
  peak RSS 36.7MB. ~3–8× slower than libvips per image; irrelevant for an upload-time operation.
- **fit-inside math parity bit-for-bit**: `DynamicImage::resize(w,h,Lanczos3)` uses the same
  `scale=min(w/ow,h/oh)` with no ≤1.0 clamp — matches sharp's default (which also allows upscaling;
  no call site passes `withoutEnlargement`). Empirical: 2000×1500 → exactly 800×600 / 512×384.
- **Metadata**: `image` encoders never carry source EXIF/ICC/XMP through — matches sharp's
  strip-by-default (threat-model S4 GPS-passthrough row is satisfied by construction, but still gets
  a machine assertion per DoD).

## libvips crate REJECTED (evidence, not taste)

- NOT cmake-blocked: its build.rs is three `rustc-link-lib` lines. It fails at **link** time here —
  `rust-lld: unable to find library -lvips / -lglib-2.0 / -lgobject-2.0` — because the system shared
  libs exist only after `apt-get install libvips-dev` (simulated: resolves cleanly, 198 pkgs / 173
  with --no-install-recommends, no cmake).
- That makes it **Docker-only viable**: `cargo build/test/clippy` would permanently fail in the dev/CI
  sandbox unless provisioning is kept in sync — the exact "works in Docker, broken locally" class this
  project already rejected with `aws-sdk-s3` (whose block IS cmake: aws-lc-sys build.rs).
- Consequence for the packet's coupled Q1×Q2×Q3: Q3 does NOT force the two-image split, and does NOT
  provide the aws-sdk-s3 presigner — Q2 must resolve on its own merits (header-signed vs hand-rolled
  query presign vs alternative flow).

## Two MUST-CARRY items for the build (found by the spike)

1. **EXIF orientation is a silent parity gap**: `image::open()` does NOT auto-apply orientation; sharp
   does, implicitly, on every profile. The port must explicitly `decoder.orientation()` →
   `apply_orientation()` on ALL THREE profiles, with a rotated-EXIF fixture test (wrong output is
   silent — no error). Named DoD item.
2. **Three sharp profiles, not two**: product `spa-proxy.ts:223-226` (800×800 q82), logo
   `themes.ts:127-130` (512×512 q80), **entry-photo `spa-proxy.ts:280` (`.rotate()` + 1024×1024 q78)**
   — the unauthenticated-upload route's profile, missing from the original two-profile framing.
