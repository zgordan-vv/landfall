export {
  SUPPORTED_EVENT_TYPES,
  SUPPORTED_SCHEMA_VERSIONS,
  checkEventCompatibility,
} from "./compatibility.js";
export type { CompatibilityErrorCode, CompatibilityResult } from "./compatibility.js";
export type * from "./generated/v1.js";
