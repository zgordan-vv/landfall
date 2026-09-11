/** Adapter-facing types intentionally contain no @solana/kit runtime imports. */
export interface SolanaBlockhashSnapshot {
  readonly blockhash: string;
  readonly lastValidBlockHeight: string;
}

export type BlockHeightInput = string | bigint;

/** Normalizes Kit's bigint/string block-height representations at the boundary. */
export function normalizeBlockhashSnapshot(blockhash: string, lastValidBlockHeight: BlockHeightInput): SolanaBlockhashSnapshot {
  const normalizedHash = blockhash.trim();
  const normalizedHeight = typeof lastValidBlockHeight === "bigint"
    ? lastValidBlockHeight.toString(10)
    : lastValidBlockHeight.trim();
  if (normalizedHash.length === 0) throw new Error("blockhash must not be empty");
  if (!/^(0|[1-9][0-9]*)$/.test(normalizedHeight)) throw new Error("lastValidBlockHeight must be a canonical unsigned decimal");
  return Object.freeze({ blockhash: normalizedHash, lastValidBlockHeight: normalizedHeight });
}

export interface SolanaRoute {
  readonly routeId: string;
  readonly endpoint?: string;
}

export interface SolanaClientPort<TPrepared, TSigned, TSignature> {
  readonly getLatestBlockhash: () => Promise<SolanaBlockhashSnapshot>;
  readonly simulate: (transaction: TPrepared) => Promise<unknown>;
  /** Signing remains application-owned; the adapter receives only the signed value. */
  readonly sign: (transaction: TPrepared) => Promise<TSigned>;
  readonly submit: (signed: TSigned, route: SolanaRoute) => Promise<TSignature>;
  readonly confirm: (signature: TSignature, blockhash: SolanaBlockhashSnapshot) => Promise<unknown>;
}

export async function captureLatestBlockhash<TPrepared, TSigned, TSignature>(client: SolanaClientPort<TPrepared, TSigned, TSignature>): Promise<SolanaBlockhashSnapshot> {
  const snapshot = await client.getLatestBlockhash();
  return normalizeBlockhashSnapshot(snapshot.blockhash, snapshot.lastValidBlockHeight);
}

/** Wraps an application operation without changing its return value or error identity. */
export async function preserveCustomerOperation<T>(operation: () => Promise<T>): Promise<T> {
  return operation();
}
