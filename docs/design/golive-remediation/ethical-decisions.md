# Ethical Decisions — Go/Live Remediation

> Recorded human decisions on ETHICAL-STOPs raised by Counsel. The conscious human is final.

## ETHICAL-STOP-1 — Retention promise vs. enforcement proof

**Red line:** "server-authoritative / UI-tells-the-truth." The #5 checkout privacy notice would display a concrete retention number (`retention_days`, "deri në {{days}} ditë") as a removal promise. The retention sweep IS scheduled and heartbeat-watched (`anonymizer-retention.ts:22-27`, `liveness-checker.ts:11`), and writes `anonymization_audit_log` with `scope='retention'` (`anonymizer/index.ts:286-289`) — BUT on a fresh pilot tenant nothing is older than `retention_days` (default 365) for ~a year, so the positive audit-row proof of an actual anonymization does not exist at launch. Day-1, the Go-gate could only prove "the retention worker is alive," not "enforcement happened."

**Options presented:**
- (a) Show the concrete number, gate on heartbeat freshness (architect default — the pulse exists).
- (b) Soften the copy to drop the specific day count; no standing numeric promise the runtime can't yet positively prove.

**DECISION: (b) — soften copy, no hard number.**
- sq: *"Të dhënat tuaja i ruajmë vetëm aq sa duhet për porositë tuaja dhe heqim të dhënat që ju identifikojnë sipas kërkesës."*
- en: *"We keep your data only as long as needed for your orders and remove the details that identify you on request."*

**Rationale:** A softened, accurate statement is honest from day one without depending on an enforcement proof that cannot exist until data ages. Removes the retention Go-gate from the launch checklist. Aligns with Counsel's grounded caveat that the day-1 gate would otherwise lean on heartbeat-presence ("worker breathes"), not enforcement.

**Consequence for implementation:** #5 i18n key `checkout.privacy.retention` drops the `{{days}}` interpolation and the numeric promise. No retention Go-gate. Copy still satisfies anonymize-not-delete + contact-the-restaurant constraints (N3). The `anonymizer-retention` heartbeat remains observable as ordinary operability, not as a launch gate.

- **Date:** 2026-06-20
- **Owner:** Owner (sviatoslavsyniak@gmail.com)
- **Decided by:** human (conscious final)

## ETHICAL-STOP-2 — Fake-success on real failure (dev-mock)

**Red line:** "UI tells the truth." `CheckoutPage.tsx:439` could navigate a *real* failed order to `o_mock_123` fake-success when a real session carries the `dos_dev` sessionStorage flag.

**DECISION: resolved by the proposal as written — no human override needed.** Compile-time `import.meta.env.DEV && isDevMode()` gate dead-strips the branch from prod builds. Lifts on the green proof: prod-bundle grep for `o_mock_123` → absent (threat-model #9). Recorded so the approval shows the false-success path was closed deliberately.

- **Date:** 2026-06-20
- **Owner:** System Architect

## Non-blocking notes carried (not red lines)

- **N2 (courtesy):** give the `sushi-durres` owner a heads-up that their live menu/branding is mirrored to staging for validation. Owner to action; not a launch blocker.
- **N5 (courier data dignity):** courier holds a stranger's name/phone/address/door-photo on-device with no governing copy. **Deferred — post-pilot, flagged** (R-N5, Owner). Counsel friction, not a STOP.
