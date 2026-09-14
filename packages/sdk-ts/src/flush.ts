export type FlushStatus = "flushed" | "timed_out" | "failed";

export interface FlushResult {
  readonly status: FlushStatus;
  readonly error?: unknown;
}

export interface FlushOptions {
  /** Maximum time allowed for the shutdown flush. Defaults to two seconds. */
  readonly timeoutMs?: number;
}

/** Runs a shutdown flush without allowing it to block process termination indefinitely. */
export async function boundedFlush(
  flushOperation: () => Promise<void>,
  options: FlushOptions = {},
): Promise<FlushResult> {
  const timeoutMs = options.timeoutMs ?? 2_000;
  if (!Number.isInteger(timeoutMs) || timeoutMs < 0)
    throw new Error("flush timeout must be a non-negative integer");

  const timerHost = globalThis as unknown as {
    setTimeout: (handler: () => void, timeout: number) => unknown;
    clearTimeout: (handle: unknown) => void;
  };
  let timer: unknown;
  const timeout = new Promise<FlushResult>((resolve) => {
    timer = timerHost.setTimeout(() => resolve({ status: "timed_out" }), timeoutMs);
  });
  const operation = Promise.resolve()
    .then(flushOperation)
    .then((): FlushResult => ({ status: "flushed" }))
    .catch((error): FlushResult => ({ status: "failed", error }));
  try {
    return await Promise.race([operation, timeout]);
  } finally {
    if (timer !== undefined) timerHost.clearTimeout(timer);
  }
}
