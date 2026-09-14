const ALLOWED_METADATA_KEYS = new Set(["service", "app_version", "route", "cluster", "network"]);

/** Removes URL userinfo, query, and fragment components before telemetry use. */
export function redactEndpoint(endpoint: string): string {
  const trimmed = endpoint.trim();
  const withoutCredentials = trimmed.replace(/^(https?:\/\/)([^/@]+@)/i, "$1");
  return withoutCredentials.replace(/[?#].*$/, "");
}

/** Keeps only bounded scalar metadata keys explicitly allowed by the SDK policy. */
export function allowMetadata(
  metadata: Record<string, unknown>,
): Record<string, string | number | boolean> {
  const result: Record<string, string | number | boolean> = {};
  for (const [key, value] of Object.entries(metadata)) {
    if (!ALLOWED_METADATA_KEYS.has(key)) continue;
    if (typeof value === "string" && value.length <= 160) result[key] = value;
    else if (typeof value === "number" && Number.isFinite(value)) result[key] = value;
    else if (typeof value === "boolean") result[key] = value;
  }
  return Object.freeze(result);
}
