# Manual SDK integration guide

Use manual events when the application does not use the Solana Kit adapter or
when a framework owns the transaction lifecycle callbacks.

```ts
import { BatchAssembler, EventBuffer, buildTraceCreated } from "@landfall/sdk";

const buffer = new EventBuffer(100);
const assembler = new BatchAssembler(100);
buffer.push(buildTraceCreated({
  projectId, environmentId, traceId, eventId,
  occurredAt: new Date().toISOString(),
  source: { kind: "application", name: "checkout", version: "1.0.0" },
}, { flow: "checkout", transaction_version: "legacy" }));

const batch = assembler.assemble(buffer);
if (batch) await sdk.flush(() => collector.send(batch));
```

Keep the buffer bounded and flush outside the customer transaction path. Use
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
