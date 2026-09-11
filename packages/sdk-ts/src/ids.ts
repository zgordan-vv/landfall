const UUID_V7 = /^[0-9a-f]{8}-[0-9a-f]{4}-7[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/;

/** Generates a canonical lowercase UUIDv7 string. */
export function generateUuidV7(nowMs: number = Date.now()): string {
  if (!Number.isSafeInteger(nowMs) || nowMs < 0) throw new Error("nowMs must be a non-negative safe integer");
  const timestamp = nowMs.toString(16).padStart(12, "0");
  let random = "";
  for (let index = 0; index < 20; index += 1) random += Math.floor(Math.random() * 16).toString(16);
  const variant = ((parseInt(random[0] ?? "0", 16) & 0x3) | 0x8).toString(16);
  const value = `${timestamp.slice(0, 8)}-${timestamp.slice(8)}-7${random.slice(1, 4)}-${variant}${random.slice(5, 8)}-${random.slice(8, 20)}`;
  return value;
}

export function isCanonicalUuidV7(value: string): boolean {
  return UUID_V7.test(value);
}
