// Hand-maintained re-export of the NAPI-RS generated binding types
// (./binding.d.ts). See index.js for why the generated output is kept in a
// separate file.
//
// `SlopDetectionResult` is re-declared explicitly (rather than relying on
// napi build's own alias generation) because it isn't stable across
// @napi-rs/cli versions -- some versions emit
// `export interface SlopDetectionResult extends NapiSlopDetectionResult {}`
// alongside the Napi-prefixed type, others emit only the Napi-prefixed type.
// Declaring it here keeps the public type name stable regardless.
import type { NapiSlopDetectionResult } from './binding'

export * from './binding'
export interface SlopDetectionResult extends NapiSlopDetectionResult {}
