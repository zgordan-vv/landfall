# Dashboard runbook

## Start

From the repository root run `npm run dev:dashboard`, then open
`http://127.0.0.1:5173/`. Production validation uses `tsc -b apps/dashboard`
followed by `apps/dashboard/node_modules/.bin/vite build`.

## Manual smoke path

1. Open `#overview` and confirm the health panel, metric cards, latency trend,
   and route coverage render without raw JSON.
2. Open `#traces`, search `swap`, and confirm the URL becomes
   `#traces?q=swap`; clear the filter and verify the empty state with an
   unmatched term.
3. Select `tr_01HZX9` and confirm `#traces/tr_01HZX9` shows lifecycle state,
   attempts, observations, execution, diagnoses, recommendations, and missing
   evidence.
4. Open `#comparison` and confirm baseline/candidate sample sizes, changes,
   missing-data rate, warning, and instrumentation coverage.
5. Navigate with keyboard only; focused links/buttons must remain visible and
   status text must not rely on color alone.

## Test evidence and limitations

The current UI uses deterministic fixture data and a lightweight hash router.
API/MSW wiring, Playwright automation, live polling, and full automated
accessibility assertions remain follow-up work; this runbook therefore proves
the composition shell and information architecture, not production data
freshness.
