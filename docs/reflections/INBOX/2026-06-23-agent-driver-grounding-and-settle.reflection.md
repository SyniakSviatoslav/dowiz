---
CONTEXT:   RSI synthetic-user driver (e2e/driver) — an LLM persona observes a page, reasons,
           and acts via Playwright, emitting findings on friction. Run against live staging on
           free OpenRouter models.
DECISIONS: First version fed the reasoner a thin observation (title + truncated a11y/DOM) and
           asked it to choose selectors; observed immediately after page.goto(domcontentloaded).
WHERE:     Round 1 produced 3 "findings" that were all NOT_A_BUG — the model invented selectors
           that do not exist (menu-button, item-card-0, "California (6)"). Round 2 produced a
           "no actions" finding — the driver observed an un-hydrated SPA shell. Both = noise,
           not signal; zero real discovery.
WHY:       Two distinct causes. (1) An agent asked to GUESS selectors from a thin observation
           hallucinates them → false friction findings. Fix: ground the observation — enumerate
           the REAL actionable elements with concrete selectors and instruct the reasoner to
           copy a selector verbatim from that list, never invent one. (2) Observing an SPA at
           domcontentloaded reads an empty shell before React hydrates → the model correctly but
           uselessly reports "nothing here". Fix: settle (networkidle + first actionable visible,
           time-boxed) before observing. After both fixes (round 3) the persona drove the real
           storefront (search + real data-testid add-to-cart), 0 false findings.
CONFIDENCE: high
NEXT-TIME: For any agentic browser driver: (a) feed grounded, real selectors and forbid invented
           ones — an LLM/script that guesses DOM targets manufactures false findings; (b) wait
           for the SPA to settle before the first observation. Treat free-tier models as weak +
           flaky (429 upstream; reasoning models return 200 with empty `content` but a filled
           `reasoning` field) — use a model fallback chain + retry on empty-200, and isolate
           per-session failures so one flaky session can't sink a round.
LINK:      e2e/driver/agent-driver.ts ; e2e/driver/reasoners.ts ; commit 68a33b41
---
