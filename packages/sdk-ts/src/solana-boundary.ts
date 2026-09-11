/** Adapter-facing types intentionally contain no @solana/kit runtime imports. */
export interface SolanaBlockhashSnapshot {
  readonly blockhash: string;
  readonly lastValidBlockHeight: string;
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

/** Wraps an application operation without changing its return value or error identity. */
export async function preserveCustomerOperation<T>(operation: () => Promise<T>): Promise<T> {
  return operation();
}
