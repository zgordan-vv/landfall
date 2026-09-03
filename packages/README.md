# TypeScript workspace boundaries

Landfall uses one pnpm workspace for deployable TypeScript applications,
reusable packages, and controlled examples. The root `package.json`,
`pnpm-workspace.yaml`, `.node-version`, and `tsconfig.base.json` are
the shared toolchain contract.

## Planned dependency direction

```text
packages/protocol-ts
        ^
        |
        +-------------------+
        |                   |
packages/sdk-ts     packages/api-client
                            ^
                            |
                     apps/dashboard

examples/* -> public package exports only
```

Rules:

- `protocol-ts` contains generated or verified event protocol types and never
  depends on the SDK, REST client, dashboard, React, or Node-only code;
- `sdk-ts` may depend on `protocol-ts`, but never on the dashboard or generated
  query client;
- `api-client` owns the generated REST surface and may depend on neutral
  protocol types when a public API explicitly reuses them;
- `dashboard` consumes `api-client` and presentation dependencies, never SDK
  internals;
- examples import package names through their public `exports`; they do not
  reach into another package's `src` directory;
- internal dependencies use the `workspace:` protocol so pnpm cannot silently
  resolve a registry package with the same name;
- TypeScript `paths` aliases must not be used to bypass package manifests or
  public exports.

Each workspace now has its own manifest, TypeScript configuration, public entry
point, and project references following this graph. All three libraries are
temporarily marked `private` at version `0.0.0` so an empty skeleton cannot be
published accidentally. A later repository check validates their dependency
fields and TypeScript project references; `just lint` runs that architecture
check before CI treats the boundary as enforced.

The dashboard skeleton contains only the React/Vite composition root and a
placeholder landmark. Product routes, styling, server-state libraries, API
calls, and business UI belong to Phase 12. Likewise, the three libraries expose
empty entry points until their canonical contracts are implemented in their
own phases.

## Runtime and package-manager contract

- `.node-version` pins the reproducible development/release runtime to Node.js
  `24.20.0`.
- `engines.node` accepts only the supported Node 24 LTS interval beginning with
  its LTS floor; release and benchmark jobs still use the exact pin.
- `packageManager` and `engines.pnpm` require pnpm `11.25.0`.
- `devEngines` plus `pmOnFail` and `runtimeOnFail` make exact development-tool
  mismatches fail instead of silently downloading or accepting another runtime.
- pnpm workspace settings make dependency engine and peer mismatches fail,
  prevent implicit peer installation, reject workspace cycles, and save future
  registry dependencies with an exact `=` version prefix.
- the single root lockfile is committed; nested lockfiles are prohibited.

## TypeScript base policy

Every TypeScript project extends `tsconfig.base.json`. The base targets ES2024
and Node-style ESM resolution, has no ambient global type packages, and enables
strict checking beyond the `strict` preset.

The most important additional checks distinguish an absent optional property
from an explicitly `undefined` value, make indexed access possibly undefined,
require every control-flow path to return, reject unused or fall-through code,
and require isolated declarations. These constraints matter for generated
event/API contracts because a type that compiles differently in two packages
is a protocol defect.

During the TypeScript 7 foundation, `just lint` combines these compiler-enforced
checks with the package-boundary architecture check. The planned ESLint React
rules will be added before application hooks and components are implemented;
an unsupported TypeScript parser is not forced past pnpm's strict peer checks.

Package-specific configs own `rootDir`, `outDir`, project references, and emit
mode. The browser dashboard additionally opts into DOM libraries and bundler
resolution; Node packages retain `NodeNext` so relative ESM imports are checked
the same way Node executes them.
