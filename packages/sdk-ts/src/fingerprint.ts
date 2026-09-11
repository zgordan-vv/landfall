import type { SignedBytesFingerprint, UuidV7 } from "@landfall/protocol";

export type HmacBytes = (serializedSignedBytes: Uint8Array) => Uint8Array;

/** Fingerprints exact serialized signed bytes; callers provide environment-keyed HMAC. */
export function fingerprintSignedBytes(keyId: UuidV7, serializedSignedBytes: Uint8Array, hmac: HmacBytes): SignedBytesFingerprint {
  if (serializedSignedBytes.byteLength === 0) throw new Error("serialized signed transaction bytes must not be empty");
  const digest = hmac(serializedSignedBytes);
  if (digest.byteLength !== 32) throw new Error("HMAC must return exactly 32 bytes");
  return Object.freeze({ algorithm: "lf-hmac-sha256-v1", key_id: keyId, value_hex: toHex(digest) });
}

function toHex(bytes: Uint8Array): string {
  return Array.from(bytes, (byte) => byte.toString(16).padStart(2, "0")).join("");
}
