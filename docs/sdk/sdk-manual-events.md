# Manual lifecycle events example

Run the example after building the SDK:

```sh
node_modules/.bin/tsc -b packages/sdk-ts --pretty false
node examples/sdk-manual-events.mjs
```

It creates trace, signing, and submission events with the typed builders,
places them in the bounded buffer, assembles one stable batch, and flushes it
through an injected operation. The example prints the resulting JSON without
including private keys or raw signed transaction bytes.

For integration guidance and privacy boundaries, see the [manual SDK
integration guide](manual-sdk-guide.md).
