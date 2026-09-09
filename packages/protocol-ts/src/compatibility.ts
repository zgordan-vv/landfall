/** Exact wire versions implemented by this package. */
export const SUPPORTED_SCHEMA_VERSIONS: readonly ["1.0"] = Object.freeze(["1.0"]);

/** Event discriminators implemented for wire version 1.0. */
export const SUPPORTED_EVENT_TYPES: readonly WireEvent["event_type"][] = Object.freeze([
  "landfall.data_quality.detected",
  "solana.blockhash.acquired",
  "solana.business_outcome.observed",
  "solana.confirmation_wait.completed",
  "solana.confirmation_wait.started",
  "solana.execution.enriched",
  "solana.signing.completed",
  "solana.signing.started",
  "solana.simulation.completed",
  "solana.simulation.started",
  "solana.status.observed",
  "solana.submission.completed",
  "solana.submission.retry_scheduled",
  "solana.submission.started",
  "solana.trace.created",
]);

/** Stable public reason for rejecting an unsupported producer capability. */
export type CompatibilityErrorCode = "LF_UNSUPPORTED_SCHEMA_VERSION" | "LF_UNSUPPORTED_EVENT_TYPE";

/** Result returned before schema selection or typed event decoding. */
export type CompatibilityResult =
  | { readonly supported: true }
  | { readonly supported: false; readonly code: CompatibilityErrorCode };

const supportedEventTypes: ReadonlySet<string> = new Set(SUPPORTED_EVENT_TYPES);

/**
 * Checks the exact advertised capability matrix without reflecting untrusted
 * version or event strings into the returned error.
 */
export function checkEventCompatibility(
  schemaVersion: string,
  eventType: string,
): CompatibilityResult {
  if (schemaVersion !== "1.0") {
    return { supported: false, code: "LF_UNSUPPORTED_SCHEMA_VERSION" };
  }
  if (!supportedEventTypes.has(eventType)) {
    return { supported: false, code: "LF_UNSUPPORTED_EVENT_TYPE" };
  }
  return { supported: true };
}
import type { WireEvent } from "./generated/v1.js";
