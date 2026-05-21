export interface NativeRuntime {
  beginShutdown(): void
  getPendingCount(): number
  isShuttingDown(): boolean
  resetForTests(): void
  runNativeFunction(fn: Function, receiver: unknown, args: unknown[]): unknown
  waitForDrain(): Promise<void>
}

export declare function createNativeRuntime(): NativeRuntime

export declare function wrapNativeAddon<T extends Record<PropertyKey, unknown>>(
  addon: T,
  runtime?: NativeRuntime,
): T

export declare function beginNativeAddonShutdown(): void
export declare function isNativeAddonShuttingDown(): boolean
export declare function waitForNativeAddonWorkToDrain(): Promise<void>
