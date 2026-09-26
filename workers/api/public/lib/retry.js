// A read that the platform refused for load is asked again (lane W-QA, 2026-09-26).
//
// The Worker answers 503 "exceeded resource limits" to a share of requests
// (the Free plan's CPU cap). Measured during the QA walk of 2026-09-26: the
// storefront's first menu read got that 503 and the customer was shown "The
// menu did not load · HTTP 503" with a retry button -- a button that would
// have worked, a second later. A READ is safe to repeat, so the storefront
// repeats it, a bounded number of times, before it shows anything.
//
// ONLY 502/503/504 and a failed connection are retried; a 4xx is an answer.
// PURE but for the `fetch` and `sleep` handed in (`retry.test.mjs`).
// ASCII QUOTES ONLY (DOWIZ-COMMON-RULES rule 11).

export const TRIES = 3;
export const WAIT_MS = 700;
const AGAIN = new Set([502, 503, 504]);

/// `fetchFn(url, init)` up to `tries` times; answers the last response (or
/// throws the last connection error). The wait grows: 700 ms, then 1400 ms.
export async function readAgain(fetchFn, url, init, { tries = TRIES, waitMs = WAIT_MS, sleep = ms => new Promise(r => setTimeout(r, ms)) } = {}){
  let last = null;
  for (let i = 0; i < tries; i++) {
    if (i) await sleep(waitMs * i);
    try {
      last = await fetchFn(url, init);
      if (!AGAIN.has(last.status)) return last;
    } catch (e) {
      if (i === tries - 1) throw e;
    }
  }
  return last;
}
