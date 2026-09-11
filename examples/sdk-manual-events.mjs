import {
  BatchAssembler,
  EventBuffer,
  LandfallSdk,
  buildSigningStarted,
  buildSubmissionStarted,
  buildTraceCreated,
} from "../packages/sdk-ts/dist/index.js";

const ids = {
  projectId: "0198ef00-0000-7000-8000-000000000100",
  environmentId: "0198ef00-0000-7000-8000-000000000200",
  traceId: "0198ef00-0000-7000-8000-000000000300",
};
const source = { kind: "application", name: "checkout-demo", version: "1.0.0" };
const context = (eventId) => ({ ...ids, eventId, occurredAt: new Date().toISOString(), source });

const sdk = new LandfallSdk({ collectorUrl: "http://localhost:8080" });
const buffer = new EventBuffer(10);
const assembler = new BatchAssembler(10);

buffer.push(buildTraceCreated(context("0198ef00-0000-7000-8000-000000000401"), {
  flow: "checkout", transaction_version: "legacy",
}));
buffer.push(buildSigningStarted(context("0198ef00-0000-7000-8000-000000000402"), {
  signing_id: "0198ef00-0000-7000-8000-000000000501", transaction_version: "legacy",
}));
buffer.push(buildSubmissionStarted(context("0198ef00-0000-7000-8000-000000000403"), {
  attempt_id: "0198ef00-0000-7000-8000-000000000601", route_id: "rpc-primary",
  attempt_sequence: 1, encoding: "base64", skip_preflight: false,
}));

const batch = assembler.assemble(buffer);
if (batch !== undefined) {
  await sdk.flush(async () => {
    console.log(JSON.stringify({ batch_id: batch.batchId, events: batch.events }, null, 2));
  });
}
