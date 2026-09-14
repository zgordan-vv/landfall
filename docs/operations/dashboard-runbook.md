# Dashboard runbook

## Start

For the complete local product, run `bash scripts/compose.sh up --build -d`,
then open `http://127.0.0.1:8080/`. The dashboard reads the live Landfall API;
create a project and paste its read token using **Get started**. For frontend
development only, `npm run dev:dashboard` serves the same application at
`http://127.0.0.1:5173/`.

## Manual smoke path

1. Open **Get started**, create a project, save its administrator token outside
   the browser, then create and paste a dashboard read token.
2. Open `#overview` and confirm live summary data render without raw JSON.
3. Open `#traces`, use an actual trace UUID, then confirm its evidence,
   diagnostics, and recommendations reflect the API response.
4. Open `#comparison` after ingesting traces in two environments. A clear
   `422` message before that is expected: there is not yet a valid comparison.
5. Navigate with keyboard only; focused links/buttons must remain visible and
   status text must not rely on color alone.

## Test evidence and limitations

The hash router is intentionally simple, while data comes from the live API.
The dashboard does not manufacture portfolio traces or comparison data. It
does not poll continuously; refresh the page to retrieve the newest API state.
