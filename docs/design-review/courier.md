# Courier app — design review

Reviewed 12 screenshots of the dowiz courier mobile/desktop app (dark soft-UI theme). Couriers operate one-handed, on the move, outdoors — so glanceability, thumb-zone CTAs, large tap targets, and sunlight contrast are weighted heavily here. Note: the two "active delivery" captures both landed on a *not-found* error state, not a live map — the real navigation surface could not be assessed and is flagged as the single biggest gap.

---

### Courier login (desktop)
- **Reads as:** Clean, centered auth card — but the primary "Hyr" button reads as disabled (low-contrast grey on grey), which is the worst possible signal on a sign-in screen.
- **Suggestions:**
  - The "Hyr" submit button → make it the brand-amber filled button (same as "Fillo Turnin"). As-is it looks disabled/inactive and undercuts confidence at the very first tap.
  - Vast empty canvas around a small card → on desktop this is fine, but ensure the card max-width and vertical centering hold; consider a subtle brand/illustration or a "what is this" line so couriers know they're in the *courier* portal, not the owner one.
  - Add a "Forgot password / reset" affordance and a visible password show/hide toggle — couriers mistype on phones constantly.
  - Language switcher (SQ/EN/UA) top-right is good, but the active SQ pill is amber while inactive ones are near-invisible; raise inactive-state contrast so the control is discoverable.
  - No loading/error state visible: define an inline error region (wrong password, locked account, offline) above the button so failures don't silently no-op.

### Courier login (mobile)
- **Reads as:** Same card scaled down well; same disabled-looking primary button problem, amplified because this is the main target on a phone.
- **Suggestions:**
  - "Hyr" → amber filled, full-width, min 48px tall, anchored so it sits in the thumb zone; this is the one action on the screen and must be the loudest element.
  - Inputs are comfortably tall — keep email keyboard type=email and password type=password with show/hide; add autofocus on email.
  - The header language pills overlap the top safe-area; nudge them down below the notch/status bar and enlarge tap targets to ~44px.
  - Consider persisting last-used language and remembering the email to cut friction on repeat shift logins.

### Courier home / tasks (desktop)
- **Reads as:** Correct offline empty-state ("Je jashtë linje") with a clear CTA to go online, but oceans of dead space below make the screen feel broken/empty.
- **Suggestions:**
  - The offline empty-state card → consider merging the "go online" action directly here instead of routing to a separate "Turni" tab. One tap to go online from the screen where a courier lands is higher-value than "Shko te Turni" → another screen → another button.
  - "Offline" status pill top-right is muted grey; make online/offline a high-contrast, color-coded toggle (grey=offline, green=online) so state is glanceable at arm's length.
  - The header is cluttered with a theme toggle, an "L ALL" filter, and 3 language pills — for a courier this is noise. Demote/relocate the theme and filter controls; couriers rarely change theme mid-shift.
  - Fill the empty body with value: today's expected demand, last shift summary, or a "you're not earning while offline" nudge — anything beats a blank 600px void.
  - The bottom tab bar is good (4 clear destinations); ensure the active-tab amber + label is consistently the only amber in the bar.

### Courier home / tasks (mobile)
- **Reads as:** The most important real-world screen — and when offline it's mostly empty with a small CTA that bounces to another tab.
- **Suggestions:**
  - Replace "Shko te Turni" (go to another tab) with an inline **"Kalo në linjë / Go online"** primary button right on this card — collapse the two-step offline→online journey into one tap.
  - Make the online/offline control a persistent, color-coded sticky element (green when online) so a courier glancing down mid-ride instantly knows whether they're receiving jobs.
  - When *online with no tasks*, design a distinct waiting state ("You're online — waiting for deliveries") with a pulse/animation, separate from the offline state; today both risk looking identical.
  - Bottom nav labels are small; verify each tab hit-area is ≥48px and that the active state survives outdoor glare (amber-on-dark should, but test sunlight legibility).

### Courier shift control (desktop)
- **Reads as:** Clear, well-structured — the amber "Fillo Turnin" is correctly the loudest element and the icon+copy explain the state. This is the strongest screen.
- **Suggestions:**
  - Good: amber filled "Fillo Turnin" is unmistakably the primary action. Keep this pattern and reuse it for "go online" on the home screen.
  - The "Mesazher për klientët" (customer messaging) card is secondary correctly, but the "—" country-code select is cryptic; pre-fill +355 and show a flag so it's obvious what to enter.
  - Add a clear *currently-online* counterpart to this screen (elapsed shift time, deliveries this shift, a big "End shift" button) so the same surface handles both start and stop with equal clarity.
  - The "Ruaj" (save) outline button is fine as secondary, but give the phone field inline validation (valid/invalid) since a wrong number breaks the customer-contact feature silently.

### Courier shift (mobile)
- **Reads as:** Excellent — big amber "Fillo Turnin", clear icon, readable copy. This is the template the rest of the app should follow.
- **Suggestions:**
  - "Fillo Turnin" is correctly full-width, tall, amber, thumb-reachable. Lock this as the system's primary-action spec.
  - The customer-messaging card sits below the fold-ish; that's fine since it's optional, but confirm the "Ruaj" button and phone field don't get covered by the keyboard when focused.
  - On going online, transition this screen into an active-shift view (timer + "End shift") rather than navigating away — couriers should manage shift state in one place.
  - Country-code "—" select → default to local country and widen the tap target.

### Courier earnings (desktop)
- **Reads as:** Solid stat-card layout (Today / This week / This month) over a payment-history empty state — clear and scannable.
- **Suggestions:**
  - "0 ALL / 0 ALL / 0 ALL" three times reads as broken until you notice they're distinct periods; emphasize the period label (SOT/KJO JAVË/KY MUAJ) and de-emphasize repeated "ALL", or show currency once.
  - Make "SOT" (today) the hero card — larger, amber-accented — since that's the number a courier checks most during a shift; the other two are reference.
  - Empty payment history is well done (icon + heading + helper). Consider adding an expected-payout date or payout-schedule line so "Nuk ka pagesa akoma" feels like *pending*, not *missing*.
  - Add a per-delivery breakdown entry point (tap a stat → see the deliveries that built it) for trust; couriers care deeply about whether every job was counted.

### Courier earnings (mobile)
- **Reads as:** Clean three-up stat grid + empty history; good information density for a phone.
- **Suggestions:**
  - Three equal "0 ALL" cards compete for attention; promote "SOT" to a full-width hero number at top, with week/month as a smaller secondary row.
  - Currency formatting: show large numerals and reduce "ALL" to a quiet suffix — the figure is what's glanced at.
  - The empty-state card has a faint dashed border that can disappear in sunlight; use a solid subtle fill instead of dashed outline for outdoor legibility.
  - Add pull-to-refresh and a "last updated" timestamp so couriers trust the figure is current mid-shift.

### Courier delivery history (desktop)
- **Reads as:** Correct empty state ("Nuk ka dërgesa akoma") with icon + helper; header shows "0 dërgesa" count. Clean but, again, lots of empty canvas.
- **Suggestions:**
  - When populated, design the row for glanceability: date, restaurant→destination, distance, payout, status pill — payout and status should be the visually dominant fields.
  - Add date-range / period filtering and a running total at top, since history doubles as an earnings audit for couriers.
  - The "0 dërgesa" count is muted; pair it with the period it covers ("0 dërgesa today / all-time") so the number has meaning.
  - Empty-state is fine; keep the package icon consistent with the earnings empty-state visual language (they currently differ slightly).

### Courier history (mobile)
- **Reads as:** Same clean empty state, well-sized for phone.
- **Suggestions:**
  - Populated rows must be tall, tappable cards (≥64px) with payout right-aligned and bold, status as a color-coded pill (delivered=green, cancelled=red).
  - Add a sticky period filter or segmented control (Today / Week / Month) at top — couriers reconcile earnings by period.
  - Consider grouping by day with a per-day subtotal so scrolling reveals daily earning rhythm.
  - Keep empty-state copy, but add a subtle CTA ("Go online to start earning") that links to the shift toggle — turn a dead end into a path.

### Courier active delivery / map (desktop)
- **Reads as:** ⚠️ Not the real screen — it rendered a *not-found* error ("Nuk u gjet / Detyra e dorezimit nuk u gjet") with a "Kthehu te detyrat" button. The actual navigation/map surface could not be reviewed.
- **Suggestions:**
  - **Highest priority:** capture and review the real active-delivery state — this is the single most safety- and revenue-critical courier screen and it's currently unrepresented.
  - The error state itself is acceptable (clear title, helper, recovery CTA) but the "Destinacioni" header with an empty body is jarring; center the error card and keep the header minimal.
  - For the *real* screen, the spec should be: a full-bleed map dominating the view; one giant primary action that changes with state (Navigate → Arrived at pickup → Picked up → Navigate to drop → Delivered); customer/restaurant contact as one-tap call buttons; and a current-status banner (assigned / picked-up / delivering) that's readable in sunlight.
  - Map controls and the primary state-action button must live in the bottom thumb zone; never put the critical "Picked up / Delivered" action at the top where it can't be reached one-handed on a bike.
  - Verify how this not-found path is reached — if couriers can land here from a stale link/expired task, give a clearer message ("This delivery was reassigned/completed") rather than a generic not-found.

### Courier active delivery (mobile)
- **Reads as:** ⚠️ Same not-found error state as desktop; the real mobile delivery/map screen — the app's most important surface — is not captured here.
- **Suggestions:**
  - **Re-capture the live delivery state** (assigned, picked-up, en-route) before any further review; this is the screen couriers spend their working time on and the one most exposed to glove/sunlight/one-handed constraints.
  - For the real screen: map full-bleed, a single state-driven primary button pinned to the bottom (huge, amber, thumb-zone), a collapsible address/customer sheet, and one-tap call + open-in-maps actions.
  - Status (assigned → picked up → delivered) must be a persistent, color-coded, large-type banner — a courier should know the current step from a half-second glance while riding.
  - The error card's dashed border + grey body is low-contrast for outdoors; if this state is kept, use solid fills and a bottom-anchored recovery button.
  - Ensure the back/recovery action ("Kthehu te detyrat") is reachable in the thumb zone, not floating mid-screen.

---

## Cross-screen themes
1. **Active-delivery screen is missing** — both captures show a not-found error, leaving the most critical courier surface unreviewed. Must be re-captured and designed against the map/thumb-zone/state-banner spec.
2. **Primary-action inconsistency** — the shift screens nail it (big amber filled button), but login renders its primary as disabled-grey and home routes its primary action to another tab. One primary-action spec (amber, filled, full-width, thumb-zone) should govern every screen.
3. **Online/offline status is under-signalled** — the muted grey "Offline" pill is the courier's most important state and is barely visible. Needs a high-contrast, color-coded (grey/green), glanceable, ideally one-tap toggle.
4. **Header clutter vs. courier focus** — theme toggle + "L ALL" filter + 3 language pills crowd the header on a tool used one-handed at speed; demote non-essential controls.
5. **Outdoor/sunlight legibility** — dashed empty-state borders and low-contrast muted greys (status pills, secondary text, inactive language pills) will wash out in daylight; favor solid fills and stronger contrast on status and primary elements.

## Top 3 highest-impact fixes
1. **Re-capture and design the live active-delivery/map screen** — full-bleed map, single bottom-anchored state-driven primary button (Navigate/Picked up/Delivered), color-coded status banner, one-tap call. This is the courier's core workspace and is currently unrepresented.
2. **Fix the primary-action language across screens** — make login "Hyr" the amber filled button (it currently looks disabled), and collapse the home offline→online flow into one inline amber "Go online" button instead of a hop to another tab.
3. **Make online/offline a loud, color-coded, glanceable toggle** — replace the muted grey "Offline" pill with a high-contrast green=online / grey=offline control reachable in one tap from the home screen.
