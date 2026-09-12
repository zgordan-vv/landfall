# Shared block-height cache

`BlockHeightCache` stores the latest `getBlockHeight` result for a bounded TTL.
Observer jobs on the same route can share the cache and avoid duplicate RPC
calls. Once stale, the next caller refreshes it through `JsonRpcClient`; the
cache stores only the numeric height and fetch timestamp.
