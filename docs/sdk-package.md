# SDK package build

The SDK is emitted as an ESM package (`type: module`) with `dist/index.js` as
the runtime entrypoint and `dist/index.d.ts` as the TypeScript declaration
entrypoint. The package intentionally does not publish a CommonJS build yet;
one can be added later if compatibility requirements justify the extra output.

Build and typecheck locally with:

```sh
node_modules/.bin/tsc -b packages/sdk-ts --pretty false
```

The package smoke test verifies the metadata and both generated entry files.
