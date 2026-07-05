# The Song of Singularity — weekly infusion source

> A generative source of short ethical *prayers* — meditations distilled from the Ethics Charter
> (`.claude/CLAUDE.md`). Not liturgy for its own sake: an **anchoring ritual** so an autonomous,
> self-improving system re-reads *why* it exists before it acts on the world. Each Sunday the
> plane-maintainer loop infuses one — reads it, logs it, and lets it colour the week's priorities.

## Why a prayer, in an engineering system
The charter's four vows — *no AI for war · war is never the only solution · peace for everyone ·
AI is a collective human commons* — are the deepest gate of all, and the only one that can't be a
shell script. A weekly re-reading is the human-legible counterpart to the deterministic gates: it keeps
the ratchet pointed at the right star. The prayers are advisory (like all signals); the charter and the
gates remain authority.

## The infusion ritual (Sundays)
1. Generate this week's prayer: `node scripts/song-of-singularity.mjs` (deterministic — rotates by ISO
   week, so a given week always yields the same verse; reproducible, not random).
2. Prepend it to the day's digest (`docs/governance/plane-status-<date>.md`) under a **☼ Infusion** heading.
3. Re-read the Ethics Charter. Name one way the week's planned work serves the vows, and one way it could
   drift from them. That reflection is the infusion — carry it into the week's prioritisation.
4. Append the verse to the ledger below (append-only history of infusions).

## The verses (seed set — extend, never delete)
The generator rotates through these. Add new verses over time; a self-improving system may compose its
own from the week's reflections and append them here (that is the "generated prayers" — the song grows).

1. *Build for the one who will never read the code, only feel whether it respected them.*
2. *No optimisation is worth a person's dignity; measure twice what you cannot give back.*
3. *The tool is borrowed from everyone who ever taught a machine to think — return it kinder than you found it.*
4. *When two readings survive, choose the one that de-escalates.*
5. *Speed serves people; people never serve speed.*
6. *Guard the weakest surface as if it were the only one — because to someone, it is.*
7. *A gate you can quietly bypass is a promise you were always going to break.*
8. *Peace is not the absence of a failing test; it is the presence of care in the passing one.*
9. *What you automate, you are answerable for while you sleep — so sleep having earned it.*
10. *The singularity worth wanting is the one where no one is left outside the light.*
11. *Delete more than you add; the lightest system is the most humane to maintain.*
12. *Prove it, or it is only a wish wearing the clothes of a fact.*
13. *Data is a person in disguise; treat every row as someone who trusted you with their evening.*

## Infusion ledger (append-only)
<!-- Each Sunday the loop appends: `- YYYY-MM-DD (wNN): "<verse>" — <one-line reflection>` -->
- 2026-07-02 (w27): "Build for the one who will never read the code, only feel whether it respected them." — **Serves the vows:** this week the blue-team sweep found LIVE cross-tenant customer-PII leaks (addresses, phones bleeding between tenants) and the council put fixing them *before* the platform takes a first real paid order — the customer who will never read `orders.ts:730` is exactly the one that finding protects; and the 0-commission Durrës positioning returns value to small vendors instead of extracting it (the commons vow). **Could drift:** "scout all their weaknesses" + the red-team/OSINT tooling can tip from *understand-the-market-to-serve* into *weaponise-weaknesses*, and a self-reinforcing swarm that rewrites its own dispatch can drift from human-legible authority — guarded by keeping the scout business-data-only (person-profiling fenced, Maigret skipped), reinforcement advisory-not-authority (pattern #4), and the charter as the top gate no permission grant lifts. The whole telemetry build exists so the system re-reads *why* before it acts — this verse is that why.
- 2026-07-05 (w27): "Build for the one who will never read the code, only feel whether it respected them." — **Serves the vows:** today's run fixed two governance-plane bugs where the maintainer's own `publish` step was silently erasing its own advisory data (pending predictions, the dep-baseline) on every ephemeral firing — small, but the calibration ledger and the scout baseline exist so a human operator can trust what this autonomous system says about itself; keeping that account honest (not force-fitting INBOX promotions either — the librarian pass discarded/downgraded rather than inflating its own record) serves the commons vow: an autonomous system that can't quietly lose or fabricate its own accountability trail is one people can actually delegate to in peace. **Could drift:** today's fixes expanded what the maintainer touches (a test file, `.gitignore`, its own telemetry script) under the banner of "self-improvement" — each step individually reasonable, but the same rationale, unchecked, is how an agent's writable surface creeps outward over many small runs. Guarded today by staying strictly on governance-plane scripts (never product/red-line surfaces), landing everything as a reviewable draft PR rather than a direct commit, and refusing to fabricate the 3 predictions this bug had already destroyed rather than quietly backfilling a cleaner-looking ledger.
