/** Small bounded FIFO buffer used before batch transport. */
export class EventBuffer<T> {
  readonly #capacity: number;
  readonly #items: T[] = [];

  constructor(capacity: number) {
    if (!Number.isInteger(capacity) || capacity < 1) throw new Error("buffer capacity must be a positive integer");
    this.#capacity = capacity;
  }

  get size(): number { return this.#items.length; }
  get capacity(): number { return this.#capacity; }

  /** Adds one item, returning false instead of growing beyond the bound. */
  push(item: T): boolean {
    if (this.#items.length >= this.#capacity) return false;
    this.#items.push(item);
    return true;
  }

  /** Removes up to `limit` oldest items for batch assembly. */
  drain(limit: number = this.#capacity): T[] {
    if (!Number.isInteger(limit) || limit < 1) throw new Error("drain limit must be a positive integer");
    return this.#items.splice(0, limit);
  }

  /** Restores an unsent batch ahead of events captured while it was in flight. */
  restoreFront(items: readonly T[]): void {
    if (items.length + this.#items.length > this.#capacity) {
      throw new Error("restored items exceed buffer capacity");
    }
    this.#items.unshift(...items);
  }
}
