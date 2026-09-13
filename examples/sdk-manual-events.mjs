import {
  LandfallSdk,
  buildSigningStarted,
  buildSubmissionStarted,
  buildTraceCreated,
  generateUuidV7,
} from "../packages/sdk-ts/dist/index.js";

const ids = {
  projectId: "0198ef00-0000-7000-8000-000000000100",
  environmentId: "0198ef00-0000-7000-8000-000000000200",
  traceId: generateUuidV7(),
};
const source = { kind: "application", name: "checkout-demo", version: "1.0.0" };
const context = () => ({ ...ids, eventId: generateUuidV7(), occurredAt: new Date().toISOString(), source });

const sdk = new LandfallSdk({ collectorUrl: process.env.LANDFALL_COLLECTOR_URL ?? "http://localhost:8080" });

sdk.capture(buildTraceCreated(context(), {
  flow: "checkout", transaction_version: "legacy",
}));
sdk.capture(buildSigningStarted(context(), {
  signing_id: "0198ef00-0000-7000-8000-000000000501", transaction_version: "legacy",
}));
sdk.capture(buildSubmissionStarted(context(), {
  attempt_id: generateUuidV7(), route_id: "0198ef00-0000-7000-8000-000000000400",
  attempt_sequence: 1, encoding: "base64", skip_preflight: false,
}));

const result = await sdk.flush();
if (result.status !== "flushed") throw result.error ?? new Error(`telemetry flush ${result.status}`);
console.log(`Delivered telemetry for trace ${ids.traceId}.`);
