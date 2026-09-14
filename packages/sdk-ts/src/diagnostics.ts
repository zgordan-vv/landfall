/** Aggregate SDK health counters. Values are intentionally bounded to counters only. */
export interface SdkHealth {
  readonly droppedEvents: number;
  readonly transportFailures: number;
}

export type HealthChangeCallback = (health: SdkHealth) => void;

export class HealthCounters {
  #droppedEvents = 0;
  #transportFailures = 0;
  readonly #onChange: HealthChangeCallback | undefined;

  constructor(onChange?: HealthChangeCallback) {
    this.#onChange = onChange;
  }

  get snapshot(): SdkHealth {
    return Object.freeze({
      droppedEvents: this.#droppedEvents,
      transportFailures: this.#transportFailures,
    });
  }

  recordDropped(count = 1): void {
    this.#droppedEvents += this.#validateCount(count);
    this.#notify();
  }

  recordTransportFailure(count = 1): void {
    this.#transportFailures += this.#validateCount(count);
    this.#notify();
  }

  #validateCount(count: number): number {
    if (!Number.isInteger(count) || count < 1)
      throw new Error("health counter increment must be a positive integer");
    return count;
  }

  #notify(): void {
    try {
      this.#onChange?.(this.snapshot);
    } catch {
      // Diagnostics must never affect customer application control flow.
    }
  }
}
