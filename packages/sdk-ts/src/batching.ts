import { EventBuffer } from "./buffer.js";
import { generateUuidV7 } from "./ids.js";

export interface OutboundBatch<T> {
  readonly batchId: string;
  readonly events: readonly T[];
}

/** Assembles bounded batches and preserves identity/order across retries. */
export class BatchAssembler<T> {
  readonly #batchSize: number;

  constructor(batchSize: number) {
    if (!Number.isInteger(batchSize) || batchSize < 1) throw new Error("batch size must be a positive integer");
    this.#batchSize = batchSize;
  }

  assemble(buffer: EventBuffer<T>): OutboundBatch<T> | undefined {
    if (buffer.size === 0) return undefined;
    return Object.freeze({ batchId: generateUuidV7(), events: Object.freeze(buffer.drain(this.#batchSize)) });
  }

  retry(batch: OutboundBatch<T>): OutboundBatch<T> {
    return Object.freeze({ batchId: batch.batchId, events: batch.events });
  }
}
