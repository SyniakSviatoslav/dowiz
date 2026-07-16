# Open-Source Credits List — dowiz / DeliveryOS + openbebop / bebop2 + hermes-agent-kernel-rewrite

Generated 2026-07-16 by a full sweep of three repos: `/root/dowiz` (product), `/root/bebop-repo`
(sovereign protocol), `/root/hermes-agent-kernel-rewrite` (dev-tooling / agent kernel). Purpose:
a gratitude list — every open-source project this codebase actually depends on, borrowed a
pattern from, or implements a spec from, so the operator can star/thank each one.

**Method**: three parallel repo sweeps (every `Cargo.toml`/`package.json`/`pyproject.toml` read,
`docs/**/*.md` + `AGENTS.md` + memory files grepped for `github.com/` and named projects) plus
direct reads of pre-existing curated attribution files this codebase already maintains:
`/root/bebop-repo/attrib/ATTRIBUTIONS.md`, `/root/bebop-repo/docs/design/LIBRARIES-FOR-STARS.md`
(an operator-authored "star these" list), `/root/dowiz/tools/skillspector/THIRD_PARTY_NOTICES.md`,
`/root/hermes-agent-kernel-rewrite/plugins/security-guidance/NOTICE`, and
`/root/hermes-agent-kernel-rewrite/optional-skills/creative/pixel-art/ATTRIBUTION.md`.

**Honesty notes carried over from source docs**:
- `docs/design/LIBRARIES-FOR-STARS.md` lists `iroh` and `zenoh` as bebop dependencies, but the
  deeper `Cargo.toml` sweep found `iroh` is an **empty feature flag, not linked** (deferred —
  version conflict with `ed25519-dalek`, see `bebop2/proto-wire/Cargo.toml:41-51`), and `zenoh`
  was a local stand-in that iroh+WSS is slated to replace. Both are real projects worth crediting
  for the design work already done around them, but neither compiles into the current binary —
  marked accordingly below.
- `LIBRARIES-FOR-STARS.md` also gives OpenCode's URL as `opencode-ai/opencode`. A separate, more
  careful piece of research in dowiz's own memory (`free-agent-research-2026-07-08.md`) found this
  is a **lookalike trap** — the real, actively-maintained OpenCode is `anomalyco/opencode`
  (transferred from `sst/opencode`); `opencode-ai/opencode` is an unrelated, archived ~13k★
  project. Corrected below.
- A handful of URLs are genuinely unconfirmed (small Termux tools, some crate-org mappings) —
  marked "URL not confirmed, verify before starring" rather than guessed.
- Where a repo's own docs used crude/offensive language unrelated to open-source credit (found in
  `attrib/ATTRIBUTIONS.md`'s "Hard bans" section), it was excluded — out of scope for this list.

---

## (A) Active dependencies

### A1. Rust crates — `/root/bebop-repo` (crates/bebop, bebop2/*, ci/crdt-fence)

| Crate | What it does | GitHub URL | How used |
|---|---|---|---|
| ratatui | Terminal UI framework | https://github.com/ratatui/ratatui (formerly ratatui-org/ratatui) | dependency — `crates/bebop` CLI |
| crossterm | Cross-platform terminal manipulation | https://github.com/crossterm-rs/crossterm | dependency — `crates/bebop` |
| clap | CLI argument parsing | https://github.com/clap-rs/clap | dependency — `crates/bebop` |
| toml (crate) | TOML config parsing | https://github.com/toml-rs/toml | dependency — `crates/bebop` |
| serde | Serialization framework | https://github.com/serde-rs/serde | dependency — used across almost every crate in all 3 repos |
| serde_json | JSON serde backend | https://github.com/serde-rs/json | dependency — widespread |
| anyhow | Ergonomic error handling | https://github.com/dtolnay/anyhow | dependency — `crates/bebop` |
| thiserror | Derive-based error types | https://github.com/dtolnay/thiserror | dependency — `crates/bebop` |
| paste | Token-pasting macro | https://github.com/dtolnay/paste | dev-dependency — `bebop2/core`, used to synthesize one `#[test]` per NIST ACVP vector |
| chacha20poly1305 | AEAD cipher (RFC 8439) | https://github.com/RustCrypto/AEADs | dependency — `crates/bebop` |
| argon2 | Password/key-derivation hashing (RFC 9106) | https://github.com/RustCrypto/password-hashes | dependency — `crates/bebop` |
| ml-kem | FIPS 203 (ML-KEM) implementation | https://github.com/RustCrypto/KEMs | dependency — `crates/bebop`; also the byte-exact reference bebop2-core's from-scratch ML-KEM was verified against |
| ml-dsa | FIPS 204 (ML-DSA) implementation | https://github.com/RustCrypto/signatures | dependency — `crates/bebop`; same dual role as ml-kem above |
| x25519-dalek | X25519 key exchange | URL not confirmed — RustCrypto lists it under RustCrypto/Curves but the canonical upstream is widely cited as dalek-cryptography/curve25519-dalek; verify before starring | dependency — `crates/bebop` |
| ed25519-dalek | Ed25519 signatures (RFC 8032) | URL not confirmed — same ambiguity as x25519-dalek (RustCrypto/signatures vs. dalek-cryptography); verify before starring | dependency — `crates/bebop`; also the classical half of the hybrid ML-DSA-65⊕Ed25519 composite signature |
| getrandom | OS randomness source | https://github.com/rust-random/getrandom | dependency — `crates/bebop` |
| signature | Signature-trait abstraction | https://github.com/RustCrypto/signatures | dependency — `crates/bebop` |
| zeroize | Secure memory zeroing | https://github.com/RustCrypto/utils | dependency — `crates/bebop`, used throughout key-handling code |
| sha2 | SHA-2 hash family | https://github.com/RustCrypto/hashes | dependency — `crates/bebop` |
| hex | Hex encode/decode | URL not confirmed — RustCrypto/utils vs. an independent `hex` crate; verify before starring | dependency — `crates/bebop` |
| tracing | Structured async-aware logging | https://github.com/tokio-rs/tracing | dependency — `crates/bebop`, dowiz `kernel`, hermes bootstrap-installer |
| tracing-subscriber | tracing output formatting/filtering | https://github.com/tokio-rs/tracing | dependency — same set as above |
| criterion | Benchmarking harness | https://github.com/bheisler/criterion.rs | dev-dependency — `crates/bebop`, dowiz `kernel` |
| tokio | Async runtime | https://github.com/tokio-rs/tokio | dependency — `bebop2/proto-wire`, `bebop2/mesh-node`, `bebop2/ports/github`, dowiz `tools/native-spa-server`, hermes `apps/bootstrap-installer` |
| tokio-tungstenite | WebSocket over tokio | https://github.com/snapview/tokio-tungstenite | dependency — `bebop2/proto-wire` (WSS transport carrier) |
| futures-util | Futures combinators | https://github.com/rust-lang/futures-rs | dependency — `bebop2/proto-wire`, `bebop2/mesh-node` |
| quinn | QUIC transport implementation | https://github.com/quinn-rs/quinn | dependency — `bebop2/proto-wire` |
| rcgen | X.509 certificate generation | https://github.com/rustls/rcgen | dependency — `bebop2/proto-wire` |
| rustls | Pure-Rust TLS | https://github.com/rustls/rustls | dependency — `bebop2/proto-wire`; also dowiz `tools/native-spa-server` (tokio-rustls) and `tools/async-spool`/`tools/telemetry/rust-spool` (via ureq's rustls+ring backend) — "operator mandate 2026-07-15: rustls with ring everywhere possible" |
| webpki-roots | Mozilla CA root bundle for rustls | https://github.com/rustls/webpki-roots | dependency — `bebop2/proto-wire` |
| tokio-rustls | rustls glue for tokio | https://github.com/rustls/tokio-rustls | dependency — `bebop2/proto-wire`; also dowiz `tools/native-spa-server` |
| http (crate) | Shared HTTP types | https://github.com/hyperium/http | dependency — `bebop2/proto-wire` |
| wasmtime | WASM runtime (Bytecode Alliance) | https://github.com/bytecodealliance/wasmtime | optional dependency, feature-gated OFF by default — `bebop2/wasm-host` |
| ring | Cryptographic primitives (constant-time HMAC-SHA256) | https://github.com/briansmith/ring | dependency — `bebop2/ports/github`, used to verify GitHub webhook HMAC signatures |
| wit-bindgen-rt | WASM Component Model bindings runtime | https://github.com/bytecodealliance/wit-bindgen | dependency — `bebop2/ports/telegram` (WASM component) |

**Vendored data**: `advisories/advisory-db-*` is a full vendored clone of **RustSec's advisory-db**
(https://github.com/rustsec/advisory-db, CC0/MIT) used by `cargo deny`/`cargo audit` per
`deny.toml`.

### A2. Rust crates — `/root/dowiz` (kernel, engine, wasm, tools/*)

| Crate | What it does | GitHub URL | How used |
|---|---|---|---|
| wasm-bindgen | Rust↔JS/WASM FFI glue | https://github.com/rustwasm/wasm-bindgen | dependency (hard-pinned `=0.2.95`) — `kernel` (optional, `wasm` feature), `wasm`, `agent-governance-wasm` |
| serde / serde_json / serde_yaml | Serialization | see A1 for serde/serde_json; serde_yaml: https://github.com/dtolnay/serde-yaml | dependency (optional, `wasm`/`pgrust` features) — `kernel` |
| sqlx | Async SQL toolkit (Postgres) | https://github.com/launchbadge/sqlx | optional dependency, `pgrust` feature — `kernel`, living-memory Postgres adapter |
| regex | Regular expressions | https://github.com/rust-lang/regex | dependency — `kernel`, L0 exact-search trigram-verify step |
| ureq | Minimal blocking HTTP client | https://github.com/algesten/ureq | dependency (rustls+ring backend) — `tools/async-spool`, `tools/telemetry/rust-spool` |
| axum | Web framework (tokio ecosystem) | https://github.com/tokio-rs/axum | dependency — `tools/native-spa-server` |
| tower-http | HTTP middleware (compression, static files) | https://github.com/tower-rs/tower-http | dependency — `tools/native-spa-server` |
| hyper-util | Hyper HTTP utilities | https://github.com/hyperium/hyper-util | dependency — `tools/native-spa-server` |
| rustls-pemfile | PEM parsing for rustls | https://github.com/rustls/pemfile | dependency — `tools/native-spa-server` |
| flate2 | DEFLATE/gzip compression | https://github.com/rust-lang/flate2-rs | dependency — `tools/native-spa-server` |
| rusqlite | SQLite bindings (bundled) | https://github.com/rusqlite/rusqlite | dependency — `tools/deep-clean`; `bundled` feature vendors the SQLite C amalgamation itself, so **SQLite** (https://sqlite.org, public domain) is a transitive credit |
| bebop2-core | PQ crypto/kernel core | https://github.com/SyniakSviatoslav/OpenBebop (or bebop.git) | cross-repo path dependency — `agent-governance-wasm` binds to the sibling bebop-repo's crypto core |

**dowiz-engine (`engine/Cargo.toml`) is deliberately zero-external-dependency** — wgpu/cosmic-text
explicitly named as out-of-scope for offline-build reasons.

### A3. Rust crates — `/root/hermes-agent-kernel-rewrite`

`hermes-kernel/kernel` is deliberately **zero-dependency** (std-only, enforced invariant). Its
`cli` crate depends only on `serde`/`serde_json` (see A1) plus the internal kernel crate.

| Crate | What it does | GitHub URL | How used |
|---|---|---|---|
| tauri | Desktop-app framework (Rust + web frontend) | https://github.com/tauri-apps/tauri | dependency — `apps/bootstrap-installer/src-tauri` |
| reqwest | HTTP client | https://github.com/seanmonstar/reqwest | dependency (rustls-tls, no OpenSSL) — bootstrap-installer |
| dirs | Cross-platform config/data dir lookup | https://github.com/dirs-dev/dirs-rs | dependency — bootstrap-installer |
| which | Locate executables in PATH | https://github.com/harryfei/which-rs | dependency — bootstrap-installer |
| once_cell | Lazy statics | https://github.com/matklad/once_cell | dependency — bootstrap-installer |
| uuid | UUID generation | https://github.com/uuid-rs/uuid | dependency — bootstrap-installer |
| windows-sys | Raw Windows API bindings | https://github.com/microsoft/windows-rs | dependency (Windows-only) — bootstrap-installer |

(`tokio`, `futures`, `serde`/`serde_json`, `anyhow`, `thiserror`, `tracing`* — see A1/A2, same
crates reused here.)

### A4. Python packages — `/root/hermes-agent-kernel-rewrite` (`pyproject.toml`)

Core, always-installed (exact-pinned as an explicit anti-supply-chain-attack policy, triggered by
the "Mini Shai-Hulud" npm/PyPI worm incident of 2026-05-12):

| Package | What it does | GitHub URL |
|---|---|---|
| openai | OpenAI API client | https://github.com/openai/openai-python |
| httpx | Async HTTP client | https://github.com/encode/httpx |
| rich | Terminal formatting | https://github.com/Textualize/rich |
| tenacity | Retry logic | https://github.com/jd/tenacity |
| pyyaml / ruamel.yaml | YAML parsing | https://github.com/yaml/pyyaml , https://sourceforge.net/projects/ruamel-yaml (mirror: https://github.com/nfebe/ruamel.yaml — verify) |
| jinja2 | Templating | https://github.com/pallets/jinja |
| pydantic | Data validation | https://github.com/pydantic/pydantic |
| prompt_toolkit | Interactive CLI prompts | https://github.com/prompt-toolkit/python-prompt-toolkit |
| croniter | Cron expression parsing | https://github.com/kiorky/croniter |
| PyJWT | JWT handling | https://github.com/jpadilla/pyjwt |
| cryptography | Crypto primitives (pyca) | https://github.com/pyca/cryptography |
| psutil | Process/system utilities | https://github.com/giampaolo/psutil |
| websockets | WebSocket client/server | https://github.com/python-websockets/websockets |
| pathspec | gitignore-style path matching | https://github.com/cpburnz/python-pathspec |
| fastapi / uvicorn | Web framework / ASGI server | https://github.com/fastapi/fastapi , https://github.com/encode/uvicorn |
| ptyprocess / pywinpty | PTY handling (Unix/Windows) | https://github.com/pexpect/ptyprocess , https://github.com/andfoy/pywinpty |
| Pillow | Image processing | https://github.com/python-pillow/Pillow |
| fire | CLI generation from Python objects | https://github.com/google/python-fire |
| certifi | Mozilla CA bundle | https://github.com/certifi/python-certifi |
| requests | HTTP client | https://github.com/psf/requests |
| urllib3 | HTTP library | https://github.com/urllib3/urllib3 |
| Markdown | Markdown-to-HTML | https://github.com/Python-Markdown/markdown |
| packaging | Version/spec parsing | https://github.com/pypa/packaging |
| python-dotenv | .env file loading | https://github.com/theskumar/python-dotenv |
| mcp | Model Context Protocol Python SDK | https://github.com/modelcontextprotocol/python-sdk |
| agent-client-protocol | ACP Python bindings | https://github.com/zed-industries/agent-client-protocol |

Lazy-installed optional extras (~40, install-on-demand per plugin/skill) — named for credit even
though not always active: `anthropic` (https://github.com/anthropics/anthropic-sdk-python),
`exa-py` (https://github.com/exa-labs/exa-py), `firecrawl-py`
(https://github.com/mendableai/firecrawl), `modal` (https://github.com/modal-labs/modal-client),
`mem0ai` (https://github.com/mem0ai/mem0), `honcho-ai` (https://github.com/plastic-labs/honcho —
see also §B, also a named design influence), `mautrix` (https://github.com/mautrix/python),
`python-telegram-bot` (https://github.com/python-telegram-bot/python-telegram-bot), `discord.py`
(https://github.com/Rapptz/discord.py), `slack-bolt`/`slack-sdk`
(https://github.com/slackapi/bolt-python , https://github.com/slackapi/python-slack-sdk),
`boto3` (https://github.com/boto/boto3), `google-auth`
(https://github.com/googleapis/google-auth-library-python), `faster-whisper`
(https://github.com/SYSTRAN/faster-whisper), `elevenlabs`
(https://github.com/elevenlabs/elevenlabs-python), `defusedxml`
(https://github.com/tiran/defusedxml — chosen specifically to block XXE on WeCom XML callbacks).
`daytona`, `hindsight-client`, `supermemory`, `nemo-relay`, `microsoft-teams-apps`,
`dingtalk-stream`, `lark-oapi` also present — URLs not independently confirmed, verify before
starring.

### A5. Node/TypeScript packages

**dowiz**:
| Package | What it does | GitHub URL |
|---|---|---|
| zod | Schema validation | https://github.com/colinhacks/zod |
| pg-boss | Postgres-backed job queue | https://github.com/timgit/pg-boss |
| pg (node-postgres) | Postgres client | https://github.com/brianc/node-postgres |
| ioredis | Redis client | https://github.com/redis/ioredis |
| jose | JWT/JOSE implementation | https://github.com/panva/jose |
| tsx | TypeScript execution | https://github.com/privatenumber/tsx |
| typescript | TypeScript compiler | https://github.com/microsoft/TypeScript |

**dowiz/tools/skillspector** wraps **NVIDIA SkillSpector**
(https://github.com/NVIDIA/skillspector, Apache-2.0) — a vulnerability-pattern scanner for
third-party agent skills/MCP servers, used as a governance gate (`docs/security/skill-scanning.md`)
before installing any new skill or MCP server; peer deps `@earendil-works/pi-ai`,
`@earendil-works/pi-coding-agent`, `typebox` — URLs not confirmed, verify before starring.

**bebop-repo** `integrations/github-webhook`: `@cloudflare/vitest-pool-workers`,
`@cloudflare/workers-types`, `vitest` (https://github.com/vitest-dev/vitest), `wrangler`
(https://github.com/cloudflare/workers-sdk).

**hermes-agent-kernel-rewrite** (large surface, headline items only):
- React 19 (https://github.com/facebook/react), `@react-three/fiber`
  (https://github.com/pmndrs/react-three-fiber), `three.js`
  (https://github.com/mrdoob/three.js), Vite (https://github.com/vitejs/vite), Tailwind CSS
  (https://github.com/tailwindlabs/tailwindcss), Vitest — `web/`
- **Ink** (https://github.com/vadimdemedes/ink, React-for-terminal-UIs) + companion packages
  (`nanostores`, `@alcalzone/ansi-tokenize`, `chalk` https://github.com/chalk/chalk,
  `wrap-ansi` https://github.com/chalk/wrap-ansi) — `ui-tui/`, effectively a fork/companion of the
  Ink ecosystem
- Electron (https://github.com/electron/electron) + `electron-builder`
  (https://github.com/electron-userland/electron-builder), CodeMirror 6
  (https://github.com/codemirror/dev), `@dnd-kit/*` (https://github.com/clauderic/dnd-kit),
  `d3-force` (https://github.com/d3/d3-force), `dompurify`
  (https://github.com/cure53/DOMPurify), `katex` (https://github.com/KaTeX/KaTeX), `mermaid`
  (https://github.com/mermaid-js/mermaid), `shiki` (https://github.com/shikijs/shiki), `node-pty`
  (https://github.com/microsoft/node-pty), `simple-git`
  (https://github.com/steveukx/git-js), Radix UI (https://github.com/radix-ui/primitives),
  TanStack Query/Virtual (https://github.com/TanStack/query,
  https://github.com/TanStack/virtual) — `apps/desktop/`
- Tauri client bindings (`@tauri-apps/*`) — `apps/bootstrap-installer/`
- **Docusaurus** (https://github.com/facebook/docusaurus) + search-local plugin
  (https://github.com/easyops-cn/docusaurus-search-local) — `website/`
- **Baileys** (https://github.com/WhiskeySockets/Baileys, reverse-engineered WhatsApp Web
  protocol library) — `scripts/whatsapp-bridge/`, a genuinely reverse-engineered protocol library
  this project depends on directly
- `spectrum-ts` (iMessage/Photon bridge SDK) — URL not confirmed, verify before starring;
  patched locally for a known upstream bug (`plugins/platforms/photon/sidecar/`)

### A6. Nix / build tooling — `/root/hermes-agent-kernel-rewrite`

| Input | What it does | GitHub URL |
|---|---|---|
| nixpkgs | Nix package collection | https://github.com/NixOS/nixpkgs |
| flake-parts | Nix flake module composition | https://github.com/hercules-ci/flake-parts |
| pyproject-nix / uv2nix / pyproject-build-systems | uv-based Python→Nix packaging | https://github.com/pyproject-nix/pyproject.nix , https://github.com/pyproject-nix/uv2nix , https://github.com/pyproject-nix/build-system-pkgs |
| npm-lockfile-fix | npm lockfile Nix helper | URL not confirmed (jeslie0 org) — verify before starring |

### A7. Vendored / forked code (literal code reuse, not just a dependency line)

- **claude-plugins-official** (https://github.com/anthropics/claude-plugins-official,
  Apache-2.0) — `hermes-agent-kernel-rewrite/plugins/security-guidance/hooks/patterns.py` is a
  **verbatim fork** of the upstream file at commit `0bde168` (25 regex/substring security rules:
  unsafe deserialization, command injection, XSS, crypto footguns, XXE, GitHub Actions injection,
  TLS-verify disablement), reproduced unmodified with an added attribution docstring. Documented
  in `plugins/security-guidance/NOTICE`.
- **pixel-art-studio** (https://github.com/Synero/pixel-art-studio, MIT) —
  `hermes-agent-kernel-rewrite/optional-skills/creative/pixel-art/scripts/palettes.py` and
  `pixel_art_video.py` port the `PALETTES` dict (23 named RGB palettes) and 12 procedural
  animation routines verbatim/near-verbatim from upstream, with documented modifications.
  Documented in `optional-skills/creative/pixel-art/ATTRIBUTION.md`.

### A8. Operational MCP servers / CLI tools (dowiz dev workflow)

| Tool | What it does | GitHub URL | Note |
|---|---|---|---|
| repowise | Codebase-intelligence MCP server (used throughout this session's tool access) | URL not confirmed (repowise.dev) — verify before starring | integrated, `.mcp.json` |
| Playwright | Browser automation/testing | https://github.com/microsoft/playwright | integrated, `@playwright/test` via MCP |
| browser-use | Browser-driving agent tool | https://github.com/browser-use/browser-use | integrated with mitigations (SkillSpector flagged 100/CRITICAL; telemetry off, BYOK LLM) |
| codebase-memory-mcp | tree-sitter + LSP structural code-graph MCP server | https://github.com/DeusData/codebase-memory-mcp | integrated (MIT), adopted 2026-07-05 for structural code queries, 54.9% measured token reduction vs. file-by-file reads |
| Headroom | Local context-compression proxy for Claude Code↔Anthropic API | https://github.com/headroomlabs-ai/headroom | integrated (Apache-2.0), live since 2026-07-07, systemd service on port 8787 |
| Hermes Agent | Standby/fallback agent framework for API outages | https://github.com/NousResearch/hermes-agent | integrated as a machine-wide fallback agent in dowiz's dev workflow; **also the literal upstream base of the entire `/root/hermes-agent-kernel-rewrite` repo** — see §B |
| OpenCode | Claude-Code-shaped open agent CLI | https://github.com/anomalyco/opencode (NOT opencode-ai/opencode — see honesty note above) | integrated as one of four complementary fallback agents |
| Aider | Git-native multi-file AI pair-programming CLI | https://github.com/Aider-AI/aider | integrated as a fallback agent (Apache-2.0) |
| Goose | Model-agnostic, MCP-native agent CLI (Block → Linux Foundation AAIF) | https://github.com/aaif-goose/goose | integrated as a fallback agent (Apache-2.0) |
| OpenHands | Autonomous coding agent (formerly All-Hands-AI) | https://github.com/OpenHands/OpenHands | integrated as a fallback agent (MIT), CLI mode, opt-in only |
| whisper.cpp | Local speech-to-text (ggml/GGUF) | https://github.com/ggml-org/whisper.cpp | used in dowiz voice-engine smoke test for sample audio, and named as bebop's local STT stack per `LIBRARIES-FOR-STARS.md` |
| RustSec advisory-db | Vulnerability advisory database for Rust crates | https://github.com/rustsec/advisory-db | vendored into `bebop-repo/advisories/`, drives `cargo audit`/`cargo deny` |
| OSV-Scanner | Google's open-source vulnerability scanner | https://github.com/google/osv-scanner | integrated in `hermes-agent-kernel-rewrite/.github/workflows/osv-scanner.yml` as a CI supply-chain gate |

---

## (B) Design/pattern influences (no code dependency)

| Source | What was borrowed | GitHub URL | License |
|---|---|---|---|
| **NousResearch/Hermes-Agent** | Skill system, AGENTS.md convention, memory-first architecture, the three customization axes (looks/narration/patrons) + key-change visibility pattern | https://github.com/NousResearch/hermes-agent | MIT — credited explicitly in bebop's `attrib/ATTRIBUTIONS.md`; also the literal package-level origin of `/root/hermes-agent-kernel-rewrite` (`pyproject.toml` author = "Nous Research", `package.json` repo = `NousResearch/Hermes-Agent.git`) |
| DietrichGebert/ponytail | "Lazy senior dev mode" skills (`/ponytail`, `/ponytail-review`, `/ponytail-audit`, `/ponytail-debt`) | https://github.com/DietrichGebert/ponytail | MIT — explicitly credited by a `Source:` line in dowiz's `AGENTS.md:74` |
| René Descartes (2×2 comparison method) | `descartes.rs` auto pro/con table | n/a (historical method, not a repo) | public domain |
| Open Science movement | `open_science.rs` reproducible-finding + citation gate | n/a (movement, not a single repo) | CC-BY |
| CasaOS (IceWhale) | `casaos.rs` bundle spec + one-command install model | https://github.com/IceWhaleTech/CasaOS | Apache-2.0 |
| SimpleMem | `simplemem.rs` 3-layer (Hot/Warm/Cold) recall model | URL not confirmed — verify before starring | MIT |
| OpenManus (MetaGLM) | `openmanus.rs` plan→todo→execute→verify loop | URL not confirmed (likely `mannaandpoem/OpenManus` or `FoundationAgents/OpenManus`) — verify before starring | MIT |
| Codex / Claude Code multi-agent fan-out | `multipilot.rs` N distinct pilots + synthesize pattern | https://github.com/anthropics/claude-code (Claude Code); OpenAI Codex CLI URL not confirmed | reference only, not a dependency |
| Karl Friston / pymdp (Active Inference, Free Energy Principle) | `active_inference.rs` policy advisor — design-grounded, explicitly not a pymdp port | https://github.com/infer-actively/pymdp | academic reference |
| OpenCode (anomalyco/opencode) | Feed/agentic-loop/TUI pattern, AGENTS.md auto-discovery | https://github.com/anomalyco/opencode | MIT — see honesty note; bebop's own doc cites the wrong URL |
| Claude Code (Anthropic) | Permission-mode design (plan/acceptEdits/bypass), headless `-p` mode | https://github.com/anthropics/claude-code | design pattern only — Claude Code itself is not a traditional open-source dependency |
| Dota 2 (Valve) | Per-match scoreboard metaphor for bebop's after-action UI | https://www.dota2.com (not a code repo) | reference-only, commercial game |
| XCOM 2 (Firaxis) | After-action-report / rewind metaphor | https://www.firaxis.com/xcom-2 (not a code repo) | reference-only, commercial game |
| DeepSeek DSpark | Speculative-decoding pattern → `src/speculate.ts` | arXiv:2607.05147 (paper, not a repo) | academic reference |
| OpenCove | Trace/ledger model | https://github.com/opencove/opencove — URL not confirmed, verify before starring | reference |
| Langfuse | Score-on-generation pattern → `governor.ts` | https://github.com/langfuse/langfuse | reference |
| ECC (affaan-m) | ReAct + correction loop | https://github.com/affaan-m/ecc | reference |
| pydantic (v2 design) | Boundary-validation-wall pattern → `src/validate.ts` | https://github.com/pydantic/pydantic | design pattern (pydantic itself is also a Python dependency, see A4, in a different codebase) |
| Honcho (plastic-labs) | Dialectic user-modeling pattern | https://github.com/plastic-labs/honcho | design influence AND active optional Python dependency in hermes-agent-kernel-rewrite (`honcho-ai==2.0.1`) |
| agentskills.io | Open standard for agent skill packaging | n/a (open standard, not a single repo) | spec compliance |
| OpenClaw | Predecessor/sibling agent framework — Hermes imports its SOUL.md/memories/skills/API-keys via `hermes claw migrate` | URL not confirmed — a 2026 breakout project (9k→210k+★ in weeks per dowiz memory notes); verify before starring | design-influence/migration-source, no code dependency found |
| s6-overlay (just-containers) | Process supervisor (PID 1) for the hermes-agent-kernel-rewrite container, replacing tini | https://github.com/just-containers/s6-overlay | fetched as pinned+SHA-verified release tarballs, not a Cargo/npm dependency |
| litellm (BerriAI) | Named as the direct cause of a dependency-pinning policy after a supply-chain compromise; subsequently **removed** as a dependency | https://github.com/BerriAI/litellm | cautionary reference — see also (D) |
| Google Wycheproof | Crypto test-vector methodology | https://github.com/google/wycheproof | planned/partial — harness skeleton exists (`bebop2/proto-crypto/src/wycheproof.rs`) but TODO bodies, not yet wired; dowiz also plans to vendor Wycheproof JSON vectors under `kat/` |
| dudect (Reparaz/Balasch/Verbauwhede methodology) | Welch's t-test constant-time verification methodology | eprint 2016/1123 (paper); reference impl https://github.com/oreparaz/dudect | design/methodology reference, `bebop2/proto-crypto/src/pq_kem.rs` |
| computer-use-linux (avifenesh) | Community-contributed MCP server integration, acknowledged in hermes-agent-kernel-rewrite README | https://github.com/avifenesh/computer-use-linux | community integration credit |
| HermesClaw (AaronWong1999) | Community-contributed WeChat bridge, acknowledged in README | https://github.com/AaronWong1999/hermesclaw | community integration credit |
| hermes-example-plugins (NousResearch) | Reference implementation for the Hermes plugin API | https://github.com/NousResearch/hermes-example-plugins | sibling reference repo |
| llama.cpp | GGUF local-inference goal architecture; specifically the `llama-cpp-2` Rust binding is marked "INTEGRATE-DIRECTLY" in dowiz's ecosystem strategy | https://github.com/ggml-org/llama.cpp | design goal, not yet built — self-host LLM infra target |
| vLLM | PagedAttention / OpenAI-compatible serving architecture goal | https://github.com/vllm-project/vllm | design goal, not yet built |
| RustCrypto/signatures | Source of vendored NIST ACVP KAT vectors used to verify bebop2-core's from-scratch ML-DSA | https://github.com/RustCrypto/signatures | verification-reference (also an active dependency in crates/bebop, see A1) |
| Automerge / cr-sqlite / Ditto / ElectricSQL / PowerSync / Zero / LiveStore / Dolt | CRDT/sync-engine architecture survey | https://github.com/automerge/automerge , https://github.com/vlcn-io/cr-sqlite , (others: URLs not individually confirmed) | evaluated as "pattern not library" — see (D), all explicitly rejected for the money/order path |

---

## (C) Specs / standards implemented

| Spec | What it is | Authority / URL | How implemented |
|---|---|---|---|
| FIPS 203 (ML-KEM) | NIST post-quantum key-encapsulation standard | https://csrc.nist.gov/pubs/fips/203/final | implemented twice: as a RustCrypto `ml-kem` dependency (`crates/bebop`) AND as a from-scratch, zero-dep reimplementation in `bebop2/core/src/pq_kem.rs`, verified against vendored NIST ACVP vectors |
| FIPS 204 (ML-DSA) | NIST post-quantum digital-signature standard | https://csrc.nist.gov/pubs/fips/204/final | same dual-implementation pattern as FIPS 203, via `ml-dsa` crate and `bebop2/core/src/pq_dsa.rs` |
| FIPS 202 (SHA-3/SHAKE/Keccak) | NIST hash-function standard | https://csrc.nist.gov/pubs/fips/202/final | from-scratch implementation, `bebop2/core/src/hash.rs` |
| FIPS 180-4 (SHA-2 family) | NIST hash-function standard | https://csrc.nist.gov/pubs/fips/180/4/final | from-scratch implementation, `bebop2/core/src/hash.rs`; also via RustCrypto `sha2` crate dependency |
| RFC 8032 (Ed25519) | Edwards-curve digital signature spec | https://www.rfc-editor.org/rfc/rfc8032 | via `ed25519-dalek` dependency + from-scratch Ed25519 in `bebop2/core` |
| RFC 9106 (Argon2) | Password-hashing/KDF spec | https://www.rfc-editor.org/rfc/rfc9106 | via `argon2` crate dependency |
| RFC 8439 (ChaCha20-Poly1305) | AEAD cipher spec | https://www.rfc-editor.org/rfc/rfc8439 | via `chacha20poly1305` crate dependency |
| NIST ACVP | Automated Cryptographic Validation Protocol — KAT test vectors | https://github.com/usnistgov/ACVP (NIST's own ACVP repo) | vendored vectors under `bebop2/core/kat/acvp/{key-gen,sig-gen,sig-ver}.json`, drive per-vector `#[test]`s |
| draft-ietf-lamps-pq-composite-sigs | Hybrid PQ+classical composite signature design | IETF draft | design target for the ML-DSA-65⊕Ed25519 hybrid signature scheme |
| draft-connolly-cfrg-xwing-kem / draft-ietf-tls-ecdhe-mlkem | Hybrid PQ+classical KEM designs (X-Wing, X25519MLKEM768) | IETF drafts | design targets cited in `bebop2/REMEDIATION-BLUEPRINT-2026-07-12.md` |
| RFC 8949/8785/7250/5705/9266/8446/4303 | CBOR, JCS, raw public keys, TLS exporters, channel binding, TLS 1.3, ESP — standards index | IETF | cited as the protocol's standards index, `bebop2/REMEDIATION-BLUEPRINT-2026-07-12.md:157` |
| NIST SP 800-90A/B/C | Random number generation standards | NIST | cited in the same standards index |
| Noise Protocol Framework | KEM-based PQ Noise KK/IK handshake pattern | https://noiseprotocol.org | design pattern (Trevor Perrin's spec, not a single GitHub repo), `bebop2` protocol handshake design |
| SPKI/SDSI (RFC 2693) | Simple public-key infrastructure / distributed security infrastructure | https://www.rfc-editor.org/rfc/rfc2693 | design reference for bebop2's capability model |
| S/Kademlia | Crypto-puzzle node IDs + α-disjoint-path DHT routing | academic paper (Baumgart & Mies 2007) | design reference, `bebop2/docs/red-team/2026-07-13/B4-architecture-decentralization.md` |
| UCAN 1.0 | User-Controlled Authorization Network delegation model | https://github.com/ucan-wg/spec | re-implemented as a from-scratch "UCAN-subset delegation" in `bebop2/proto-cap/src/roster.rs` — see also (D), full UCAN rejected as heavier than needed |
| Model Context Protocol (MCP) | Anthropic's tool/context protocol for LLM agents | https://github.com/modelcontextprotocol | implemented via the `mcp` Python SDK dependency (hermes) and used operationally in dowiz (`.mcp.json`) and bebop (`docs/integrations/mcp.md`) |
| Agent Client Protocol (ACP) | Editor↔agent protocol (Zed Industries) | https://github.com/zed-industries/agent-client-protocol | implemented via `agent-client-protocol` dependency + `acp_adapter/` in hermes-agent-kernel-rewrite |
| Developer Certificate of Origin 1.1 | Contribution sign-off standard | https://developercertificate.org | used verbatim for the contribution process, `/root/dowiz/DCO`, `CONTRIBUTING.md` |
| OpenStreetMap data model (`building=*`, `building:levels`/`height`) | Open geographic data schema | https://www.openstreetmap.org / https://github.com/openstreetmap | parsed into the kernel's own graph structure for the address-picker floor-slice feature; not a vendored library |
| Overture Maps buildings theme | Open geographic building-footprint dataset | https://github.com/OvertureMaps | named alongside OSM as a footprint/levels data source in the Gaussian-Splatting address-picker design |

---

## (D) Evaluated-but-not-adopted (still worth acknowledging the research)

| Project | What it is | GitHub URL | Why not adopted |
|---|---|---|---|
| graphdeco-inria/gaussian-splatting | The canonical/original 3D Gaussian Splatting reference implementation | https://github.com/graphdeco-inria/gaussian-splatting | "permanently rejected" as a shipped dependency — non-commercial research-only license; credited as the original research the whole splatting design cluster is downstream of |
| mosure/bevy_gaussian_splatting | WASM-viable Gaussian Splatting renderer, Apache-2.0 | https://github.com/mosure/bevy_gaussian_splatting | chosen as the primary target dependency in design docs, but **not yet present in any Cargo.toml** — planned, not shipped |
| KeKsBoTer/web-splat | WebGPU-only Gaussian Splatting viewer | https://github.com/KeKsBoTer/web-splat | evaluated, rejected for being WebGPU-only (excludes budget devices) |
| ArthurBrussee/brush | Rust-native Gaussian Splatting trainer (Burn+CubeCL) | https://github.com/ArthurBrussee/brush | evaluated as a candidate second-tier renderer/trainer, kept as backup option, not primary |
| nerfstudio-project/gsplat | Mature PyTorch Gaussian Splatting reference implementation | https://github.com/nerfstudio-project/gsplat | used server-side inside a disposable container (PyTorch+gsplat), not a Rust dependency |
| city-super/Octree-GS | Octree-based Gaussian Splatting variant | https://github.com/city-super/Octree-GS | evaluated in the research survey, not adopted |
| COLMAP | Structure-from-Motion / Multi-View Stereo pipeline | https://github.com/colmap/colmap | targeted as an out-of-process container tool ("no mature Rust SfM exists"), not a Rust dependency |
| iroh | QUIC-based P2P transport | https://github.com/n0-computer/iroh | named dependency in bebop's own `LIBRARIES-FOR-STARS.md`, but confirmed via `Cargo.toml` to be an **empty, unlinked feature flag** — deferred due to an `ed25519-dalek` version conflict and an offline-build constraint |
| Eclipse Zenoh | Pub/sub messaging middleware | https://github.com/eclipse-zenoh/zenoh | evaluated at length (scored 9/10 in bebop's own research), used only as a local-process stand-in; being replaced by iroh+WSS per `proto-wire/Cargo.toml` comments |
| RISC Zero zkVM | Zero-knowledge virtual machine | https://github.com/risc0/risc0 | evaluated, deferred to a later "money boundary" phase |
| UCAN (full spec) | User-Controlled Authorization Network | https://github.com/ucan-wg/spec | rejected as "heavier than proto-cap"; a subset was re-implemented from scratch instead (see C) |
| macaroons | Decentralized authorization tokens | (Google Research paper/format, no single canonical repo) | rejected as a capability format in favor of UCAN-subset+roster |
| Verifiable Credentials / JSON-LD | W3C credential format | https://www.w3.org/TR/vc-data-model/ | rejected as a capability format |
| JCS (RFC 8785) | JSON Canonicalization Scheme | https://www.rfc-editor.org/rfc/rfc8785 | rejected for signing-input canonicalization (float/ordering footguns); `serde_json` itself also rejected as a canonical signing format for the same reason |
| Terraform | Infrastructure-as-code tool | https://github.com/hashicorp/terraform | rejected in favor of OpenTofu |
| OpenTofu | Open-source Terraform fork | https://github.com/opentofu/opentofu | **adopted** in its place — arguably belongs in (A)/(B) as the chosen tool, listed here for contrast |
| pgBackRest | Postgres backup tool | https://github.com/pgbackrest/pgbackrest | marked "DEAD (archived Apr 2026)" in the research doc, WAL-G used instead |
| Kubernetes | Container orchestration | https://github.com/kubernetes/kubernetes | explicitly rejected repo-wide — "zero-OCI rule architecturally excludes it" |
| Kata Containers | VM-isolated container runtime | https://github.com/kata-containers/kata-containers | rejected as an OCI wrapper, conflicts with the zero-OCI mandate |
| litellm | Multi-provider LLM proxy library | https://github.com/BerriAI/litellm | **removed** as a dependency in hermes-agent-kernel-rewrite after a March-2026 supply-chain compromise disclosed in `github.com/BerriAI/litellm/issues/24512` |
| Automerge / cr-sqlite / Ditto / ElectricSQL / PowerSync / Zero / LiveStore | CRDT-based local-first sync engines | https://github.com/automerge/automerge , https://github.com/vlcn-io/cr-sqlite (others unconfirmed) | all rejected for the money/order path — "CRDT-guarantees-commutative-convergence but convergence≠obeys-legal-transitions… money doesn't commute"; structurally banned from reaching those crates via `ci/crdt-fence` |
| Dolt | Git-for-data / content-defined-chunking database | https://github.com/dolthub/dolt | evaluated in the sync-engine survey, not adopted |
| wgpu / bevy / candle / cudarc / cosmic-text | GPU/ML Rust ecosystem | https://github.com/gfx-rs/wgpu , https://github.com/bevyengine/bevy , https://github.com/huggingface/candle , https://github.com/coreylowman/cudarc , https://github.com/pop-os/cosmic-text | explicitly out-of-scope for dowiz's offline-build constraint; confirmed absent from every Cargo.toml |
| firecracker-microvm/firecracker | AWS's microVM hypervisor | https://github.com/firecracker-microvm/firecracker | evaluated for Docker→microVM migration research, not yet adopted |
| wasmtime / WasmEdge / Spin / wasmCloud / Wasmer / Wassette / Extism | WASM runtime ecosystem survey | https://github.com/bytecodealliance/wasmtime (others: WasmEdge/WasmEdge, fermyon/spin, wasmCloud/wasmCloud, wasmerio/wasmer, microsoft/wassette, extism/extism) | wasmtime is an actual optional dependency in bebop2 (see A1); the rest were evaluated in the docker-swap research and not adopted |
| Medusa / Enatega / TastyIgniter / roshanx0/restaurant-ordering-saas | Open-source food-delivery platforms | https://github.com/medusajs/medusa , https://github.com/EnateGa/Enatega-Multivendor-Food-Delivery-Solution (others unconfirmed) | evaluated as competitive/architectural reference material in `docs/research/`, not as dependencies |

---

## Summary counts

- **(A) Active dependencies**: ~150 distinct named projects/tools (Rust crates, Python packages,
  Node packages, Nix inputs, vendored code, and operational MCP/CLI tools combined). This is an
  approximate count — some projects (e.g. `serde`, `tokio`, `tracing`) appear in multiple manifests
  across the three repos and were counted once per manifest-table but represent one project.
- **(B) Design/pattern influences**: 32 distinct named projects/sources.
- **(C) Specs/standards implemented**: 21 distinct named specs/standards bodies.
- **(D) Evaluated-but-not-adopted**: 26 distinct named projects.
