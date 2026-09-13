# Manual SDK integration guide

Use manual events when the application does not use the Solana Kit adapter or
when a framework owns the transaction lifecycle callbacks.

```ts
import { LandfallSdk, buildTraceCreated, generateUuidV7 } from "@landfall/sdk";

const sdk = new LandfallSdk({ collectorUrl: "https://landfall.internal" });
const traceId = generateUuidV7();
sdk.startTrace(traceId).emit(buildTraceCreated({
  projectId, environmentId, traceId, eventId: generateUuidV7(),
  occurredAt: new Date().toISOString(),
  source: { kind: "application", name: "checkout", version: "1.0.0" },
}, { flow: "checkout", transaction_version: "legacy" }));

const result = await sdk.flush();
if (result.status !== "flushed") console.warn("telemetry was retained for retry", result);
```

`LandfallSdk` keeps a bounded FIFO buffer and posts JSON batches to
`POST /v1/ingest`. It retries temporary transport/server failures with the same
batch identity. A rejected or failed batch is restored ahead of events captured
while the request was in flight, so a later `flush()` delivers the original
order. Keep `flush()` outside the customer transaction path. Use
the typed builders instead of hand-written event JSON so enum names, decimal
strings, and required envelope fields stay compatible with the protocol.

Never place private keys, seed phrases, raw signed bytes, authorization tokens,
or full RPC URLs in event payloads. For signing evidence, use the SDK's
fingerprint helper on the exact serialized bytes and discard the transient
copy. The complete runnable example is
[`examples/sdk-manual-events.mjs`](../../examples/sdk-manual-events.mjs).

Build and typecheck with:

```sh
node_modules/.bin/tsc -b packages/sdk-ts --pretty false
node examples/sdk-manual-events.mjs
```
