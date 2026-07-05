// Lighthouse CI config — Sense 4 of the Non-Pixel Verification Net.
// Runs against the DEPLOYED staging storefront (LHCI needs a live URL; it cannot
// audit a fresh local build dir). Wired as a post-deploy / scheduled CI job, not
// a fast PR gate (size-limit + storefront-graph-guard are the PR-blocking gates).
module.exports = {
  ci: {
    collect: {
      // /s/demo is the live, data-bearing demo (Dubin & Sushi, Durrës). The
      // slug 'sushi-durres' renders an empty fallback shell (info/menu 404) —
      // do NOT audit it (the Non-Pixel net caught this on its first sweep).
      url: [
        'https://dowiz-staging.fly.dev/s/demo',
        'https://dowiz-staging.fly.dev/s/demo/checkout',
      ],
      numberOfRuns: 3,
      settings: {
        preset: 'desktop',
      },
    },
    assert: {
      assertions: {
        'categories:performance': ['error', { minScore: 0.8 }],
        'categories:accessibility': ['error', { minScore: 0.9 }],
        'largest-contentful-paint': ['error', { maxNumericValue: 2500 }],
        'cumulative-layout-shift': ['error', { maxNumericValue: 0.1 }],
        'total-blocking-time': ['error', { maxNumericValue: 300 }],
      },
    },
    upload: {
      target: 'temporary-public-storage',
    },
  },
};
