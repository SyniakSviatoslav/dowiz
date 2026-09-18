// Money, once, for every surface.
//
// THIS EXISTED THREE TIMES AND TWO OF THEM WERE WRONG. The storefront formatted
// with the customer's locale; the owner console and the courier app each had
// their own one-line `Intl.NumberFormat` with `currency:'ALL'` HARDCODED, so a
// venue trading in anything else showed lek to its owner and to its couriers
// while showing the right currency to its customers. Three copies of a rule is
// three chances to disagree, and money is the one place a disagreement is a
// support call.
//
// THE VENUE'S CURRENCY IS THE ONLY ONE THAT IS AUTHORITATIVE. An order is
// priced, totalled, charged and refunded in it; the kernel's money law is exact
// integer arithmetic in ONE currency and a second one inside it would be a
// rounding error with a customer's name on it. Converting here is a READING of
// a price, offered because a visitor paying in euros wants to know roughly what
// they are spending — never a second ledger. Every surface that shows a
// converted figure must also say what will actually be charged.
/// Minor units per major unit. Lek has none; the euro and the dollar have two.
export const DECIMALS = { ALL: 0, EUR: 2, USD: 2 };
/// The currencies this product renders. Adding one means adding its decimals
/// above and its name to the rates endpoint's short list — deliberately both,
/// so a currency cannot be half-supported.
export const CURRENCIES = ['ALL', 'EUR', 'USD'];
export function decimalsOf(code) {
    return DECIMALS[code] ?? 2;
}
/// Convert an integer amount from one currency to another, in integers.
///
/// `ppm` is parts per million of the TARGET per one unit of the base, which is
/// what `/api/public/rates` ships. The arithmetic stays whole all the way
/// through: scale to the target's minor units first, then divide once, then
/// round half-up — so no surface has to agree with another about where the
/// rounding happened, because there is only one place it can.
export function convert(amount, ppm, fromCode, toCode) {
    if (!Number.isFinite(amount))
        return 0;
    // `typeof ppm !== 'number'` rather than `Number.isFinite(ppm)` alone: the
    // two agree at runtime, but only the first one tells the compiler that `ppm`
    // is a number below. That is not a formality -- this function is reached with
    // `undefined` whenever a rate is missing, which is the common case offline.
    if (fromCode === toCode || typeof ppm !== 'number' || !Number.isFinite(ppm)) {
        return Math.round(amount);
    }
    const scale = 10 ** (decimalsOf(toCode) - decimalsOf(fromCode));
    // `amount * ppm` is at most ~1e13 for any realistic price, well inside the
    // 2^53 a JS integer is exact to, so this does not lose a unit.
    const numerator = Math.round(amount * ppm * scale);
    return Math.round(numerator / 1_000_000);
}
/// Format an integer amount of `code` for `locale`.
///
/// The amount is in MINOR units for a currency that has them, so 981 EUR-cents
/// renders as €9.81 and 900 lek renders as 900 L. Intl is told the exact number
/// of digits rather than left to guess, because its default for ALL is two and
/// a lek has none — which is how a price became "900.00 ALL" on one surface.
export function format(amount, code, locale) {
    const d = decimalsOf(code);
    const major = d === 0 ? (amount || 0) : (amount || 0) / 10 ** d;
    try {
        return new Intl.NumberFormat(locale || 'sq', {
            style: 'currency',
            currency: code,
            minimumFractionDigits: d,
            maximumFractionDigits: d,
        }).format(major);
    }
    catch {
        // An unknown code must not take the page down with it.
        return `${major.toFixed(d)} ${code}`;
    }
}
/// Build the formatter a surface actually calls.
///
/// Returns `money(amount)` — the venue's own integer amount in, a string out,
/// converted for display when a different currency is selected. Surfaces keep
/// calling one function with one argument, exactly as before; what changed is
/// that there is now one of it.
export function formatter({ base, display, rates, locale }) {
    const want = display || base;
    const ppm = rates && rates.ppm ? rates.ppm[want] : undefined;
    // NO RATE MEANS SHOW THE REAL CURRENCY, NOT A MADE-UP NUMBER. Falling through
    // to the identity conversion rendered 900 lek as "€9.00" — a figure that is
    // wrong by two orders of magnitude and looks entirely plausible, on the one
    // screen where being believed is the problem. When the rate is missing the
    // honest answer is the price the venue actually charges.
    const usable = want === base || Number.isFinite(ppm);
    const to = usable ? want : base;
    const rate = usable ? ppm : undefined;
    return amount => format(convert(amount || 0, rate, base, to), to, locale);
}
/// Fetch the rates, and never let that failure reach the page.
///
/// A storefront that cannot render a price because a currency API is slow is
/// worse than one that shows lek, so the caller gets the identity rate and a
/// `stale` flag rather than an exception.
export async function loadRates(base, apiBase = '/api') {
    try {
        const r = await fetch(`${apiBase}/public/rates?base=${encodeURIComponent(base)}`);
        if (!r.ok)
            throw new Error('rates ' + r.status);
        return await r.json();
    }
    catch {
        return { base, ppm: { [base]: 1_000_000 }, decimals: DECIMALS, stale: true };
    }
}
/// What the viewer last chose, per browser.
///
/// A display preference, never anything the server trusts: the price that is
/// charged is decided by the venue's currency on the server, so the worst a
/// tampered value does is render a wrong estimate to the person who tampered.
const KEY = 'dowiz.currency';
export function preferred(base) {
    try {
        const v = localStorage.getItem(KEY);
        if (v && CURRENCIES.includes(v))
            return v;
    }
    catch { /* private mode, blocked storage — fall through */ }
    return base;
}
export function remember(code) {
    try {
        localStorage.setItem(KEY, code);
    }
    catch { /* nothing to do; the choice simply does not persist */ }
}
