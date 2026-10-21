/**
 * Demo scenarios. Every scenario performs real RPC calls, real signing, and real
 * waiting against a devnet or local-validator cluster, then records exactly what
 * happened. Failures are produced by genuinely misconfigured transactions (too
 * little compute, overdrawn balance, stale blockhash, unreachable route), never
 * by fabricating outcomes.
 */
import { buildSignedTransfer, RpcError, sleep, TransportError } from "./solana.mjs";
import {
  TraceRecorder,
  classifySubmissionError,
  elapsedNs,
  fingerprint,
  normalizeTransactionError,
  refineWithLogs,
  uuidV7,
} from "./landfall.mjs";

const COMMITMENT_RANK = { processed: 1, confirmed: 2, finalized: 3 };
const HEALTHY_LAMPORTS = 1_000;
const SETTLEMENT_MEMO = "landfall demo settlement ".repeat(12).slice(0, 300);

/** Scenario catalog: name -> plan factory. */
export const SCENARIOS = {
  healthy: () => ({
    flow: "checkout-transfer",
    simulate: true,
    computeUnitLimit: 2_000,
    computeUnitPrice: 1_000,
  }),
  "simulation-error": () => ({
    flow: "checkout-transfer",
    simulate: true,
    overdraw: true,
    abortOnSimulationError: true,
  }),
  "rpc-rejection": () => ({ flow: "payout-transfer", overdraw: true, skipPreflight: false }),
  "execution-error": () => ({ flow: "payout-transfer", overdraw: true, skipPreflight: true }),
  // The memo program's cost grows with memo length (~107k CU for 300 bytes).
  "compute-exhaustion": () => ({
    flow: "swap-settlement",
    memo: SETTLEMENT_MEMO,
    computeUnitLimit: 60_000,
    skipPreflight: true,
  }),
  "tight-compute": () => ({
    flow: "swap-settlement",
    simulate: true,
    memo: SETTLEMENT_MEMO,
    computeUnitLimit: 120_000,
  }),
  "stale-blockhash": () => ({
    flow: "batch-payout",
    waitForExpiryBeforeSubmit: true,
    skipPreflight: true,
    replaceAfterExpiry: true,
  }),
  "slow-signer": () => ({
    flow: "custody-withdrawal",
    simulate: true,
    signingDelayMs: 21_000,
    computeUnitLimit: 2_000,
  }),
  "client-timeout": () => ({
    flow: "checkout-transfer",
    simulate: true,
    confirmTimeoutMs: 150,
    computeUnitLimit: 2_000,
  }),
  "flaky-route-retry": () => ({
    flow: "checkout-transfer",
    routes: ["flaky", "flaky", "primary"],
    computeUnitLimit: 2_000,
  }),
};

/** Runs one scenario; returns a short summary per trace it produced. */
export async function runScenario(name, ctx, environmentId) {
  const factory = SCENARIOS[name];
  if (!factory) throw new Error(`unknown scenario ${name}`);
  const plan = factory();
  const businessActionId = uuidV7();
  const first = await runTransaction(ctx, {
    ...plan,
    environmentId,
    businessActionId,
    scenario: name,
  });
  const results = [first];
  if (plan.replaceAfterExpiry && first.landing === "expired") {
    // The application rebuilds the same business action with a fresh blockhash.
    results.push(
      await runTransaction(ctx, {
        flow: plan.flow,
        simulate: true,
        computeUnitLimit: 2_000,
        environmentId,
        businessActionId,
        scenario: `${name}:replacement`,
      }),
    );
  }
  return results;
}

async function runTransaction(ctx, plan) {
  const { rpc, routes, payer, recipient, post, projectId, log } = ctx;
  const trace = new TraceRecorder({
    post,
    projectId,
    environmentId: plan.environmentId,
    businessActionId: plan.businessActionId,
    streaming: ctx.streaming,
  });
  const summary = { scenario: plan.scenario, traceId: trace.traceId, landing: "not_observed" };
  trace.emit("solana.trace.created", { flow: plan.flow, transaction_version: "legacy" });

  // 1. Blockhash.
  let started = process.hrtime.bigint();
  const latest = await rpc("getLatestBlockhash", [{ commitment: "confirmed" }]);
  const { blockhash, lastValidBlockHeight } = latest.value;
  trace.emit("solana.blockhash.acquired", {
    route_id: routes.primary.id,
    result: "acquired",
    duration_ns: elapsedNs(started),
    recent_blockhash: blockhash,
    last_valid_block_height: String(lastValidBlockHeight),
    context_slot: String(latest.context.slot),
  });

  let lamports = HEALTHY_LAMPORTS;
  if (plan.overdraw) {
    const balance = await rpc("getBalance", [payer.address, { commitment: "confirmed" }]);
    lamports = balance.value + 1_000_000_000;
  }
  const build = () =>
    buildSignedTransfer({
      payer,
      recipient,
      lamports,
      recentBlockhash: blockhash,
      computeUnitLimit: plan.computeUnitLimit,
      computeUnitPriceMicroLamports: plan.computeUnitPrice,
      memo: plan.memo,
    });

  // 2. Simulation (application-side preflight).
  if (plan.simulate) {
    const simulationId = uuidV7();
    trace.emit("solana.simulation.started", {
      simulation_id: simulationId,
      route_id: routes.primary.id,
      commitment: "confirmed",
      replace_recent_blockhash: false,
      sig_verify: false,
    });
    started = process.hrtime.bigint();
    const tx = build();
    let simulation;
    try {
      simulation = (
        await rpc("simulateTransaction", [
          tx.base64,
          { encoding: "base64", commitment: "confirmed", sigVerify: false },
        ])
      ).value;
    } catch (error) {
      trace.emit("solana.simulation.completed", {
        simulation_id: simulationId,
        route_id: routes.primary.id,
        duration_ns: elapsedNs(started),
        transport_result: error instanceof TransportError ? error.kind : "response_received",
        rpc_result: error instanceof TransportError ? "not_observed" : "node_error",
      });
      throw error;
    }
    const simError = refineWithLogs(normalizeTransactionError(simulation.err, tx), simulation.logs);
    trace.emit("solana.simulation.completed", {
      simulation_id: simulationId,
      route_id: routes.primary.id,
      duration_ns: elapsedNs(started),
      transport_result: "response_received",
      rpc_result:
        simulation.err === null
          ? "succeeded"
          : simulation.err === "BlockhashNotFound"
            ? "blockhash_not_found"
            : "execution_error",
      ...(simulation.unitsConsumed === undefined || simulation.unitsConsumed === null
        ? {}
        : { units_consumed: String(simulation.unitsConsumed) }),
      logs_present: Array.isArray(simulation.logs) && simulation.logs.length > 0,
      ...(simError ? { error: simError } : {}),
    });
    if (simulation.err !== null && plan.abortOnSimulationError) {
      trace.emit("solana.business_outcome.observed", {
        outcome: "failure",
        reason: "simulation_failed",
      });
      await trace.flush({ final: true });
      summary.landing = "not_submitted";
      log(`${plan.scenario}: simulation failed (${simError?.category}); not submitted`, summary);
      return summary;
    }
  }

  // 3. Signing (with an optional slow-signer delay, measured as real wall time).
  const signingId = uuidV7();
  trace.emit("solana.signing.started", {
    signing_id: signingId,
    transaction_version: "legacy",
    required_signatures: 1,
  });
  started = process.hrtime.bigint();
  if (plan.signingDelayMs) await sleep(plan.signingDelayMs);
  const signed = build();
  trace.emit("solana.signing.completed", {
    signing_id: signingId,
    duration_ns: elapsedNs(started),
    result: "completed",
    signature: signed.signature,
    signed_bytes_fingerprint: fingerprint(ctx.fingerprintKeyId, ctx.fingerprintKey, signed.wire),
  });
  await trace.flush();

  // 4. Optionally hold the signed transaction until its blockhash expires.
  if (plan.waitForExpiryBeforeSubmit) {
    log(
      `${plan.scenario}: holding signed transaction until block height passes ${lastValidBlockHeight}`,
    );
    await waitForBlockHeight(rpc, lastValidBlockHeight + 1, 240_000);
  }

  // 5. Submission attempts, possibly across routes with retries.
  const routeSequence = plan.routes ?? ["primary"];
  const skipPreflight = plan.skipPreflight ?? false;
  let accepted;
  let previousAttemptId;
  for (let index = 0; index < routeSequence.length; index += 1) {
    const route = routes[routeSequence[index]];
    const attemptId = uuidV7();
    if (previousAttemptId) {
      trace.emit("solana.submission.retry_scheduled", {
        previous_attempt_id: previousAttemptId,
        retry_sequence: index,
        delay_ns: "500000000",
        reason: "transport_timeout",
        next_route_id: route.id,
      });
      await sleep(500);
    }
    trace.emit("solana.submission.started", {
      attempt_id: attemptId,
      route_id: route.id,
      attempt_sequence: index + 1,
      encoding: "base64",
      skip_preflight: skipPreflight,
      preflight_commitment: "confirmed",
      max_retries: 0,
    });
    started = process.hrtime.bigint();
    try {
      const signature = await route.rpc("sendTransaction", [
        signed.base64,
        { encoding: "base64", skipPreflight, preflightCommitment: "confirmed", maxRetries: 0 },
      ]);
      trace.emit("solana.submission.completed", {
        attempt_id: attemptId,
        route_id: route.id,
        duration_ns: elapsedNs(started),
        transport_result: "response_received",
        rpc_result: "accepted",
        signature,
      });
      accepted = signature;
      break;
    } catch (error) {
      if (error instanceof TransportError) {
        trace.emit("solana.submission.completed", {
          attempt_id: attemptId,
          route_id: route.id,
          duration_ns: elapsedNs(started),
          transport_result: error.kind,
          rpc_result: "not_observed",
          error: { category: error.kind === "timeout" ? "transport_timeout" : "connection_error" },
        });
        previousAttemptId = attemptId;
        continue;
      }
      if (!(error instanceof RpcError)) throw error;
      const classified = classifySubmissionError(error, signed);
      trace.emit("solana.submission.completed", {
        attempt_id: attemptId,
        route_id: route.id,
        duration_ns: elapsedNs(started),
        transport_result: "response_received",
        rpc_result: classified.rpcResult,
        error: classified.error,
      });
      break;
    }
  }
  await trace.flush();

  if (!accepted) {
    trace.emit("solana.business_outcome.observed", {
      outcome: "failure",
      reason: "submission_failed",
    });
    await trace.flush({ final: true });
    summary.landing = "not_submitted";
    log(`${plan.scenario}: submission did not succeed`, summary);
    return summary;
  }

  // 6. Application confirmation wait (may time out by design in client-timeout).
  const waitId = uuidV7();
  const confirmTimeoutMs = plan.confirmTimeoutMs ?? 30_000;
  trace.emit("solana.confirmation_wait.started", {
    wait_id: waitId,
    commitment: "confirmed",
    timeout_ns: String(BigInt(confirmTimeoutMs) * 1_000_000n),
    search_transaction_history: true,
  });
  started = process.hrtime.bigint();
  const waited = await pollStatus(rpc, accepted, "confirmed", confirmTimeoutMs, 300);
  trace.emit("solana.confirmation_wait.completed", {
    wait_id: waitId,
    duration_ns: elapsedNs(started),
    result: waited ? "commitment_reached" : "timeout",
    ...(waited?.confirmationStatus ? { observed_commitment: waited.confirmationStatus } : {}),
  });
  await trace.flush();

  // 7. Independent observation: status, execution metadata, finality or expiry.
  const observed = await observe(ctx, trace, accepted, lastValidBlockHeight, signed);
  summary.landing = observed.landing;
  summary.signature = accepted;
  const outcome =
    observed.execution === "success"
      ? { outcome: "success" }
      : observed.execution === "failure"
        ? { outcome: "failure", reason: "execution_failed" }
        : observed.landing === "expired"
          ? { outcome: "failure", reason: "expired" }
          : { outcome: "unknown" };
  trace.emit("solana.business_outcome.observed", outcome);
  await trace.flush({ final: true });
  log(`${plan.scenario}: ${observed.landing}/${observed.execution ?? "unknown"}`, summary);
  return summary;
}

async function observe(ctx, trace, signature, lastValidBlockHeight, signed) {
  const { rpc, observerSourceId } = ctx;
  const deadline = Date.now() + 300_000;
  let best;
  let execution;
  while (Date.now() < deadline) {
    check: {
      const startedAt = process.hrtime.bigint();
      const statuses = await rpc("getSignatureStatuses", [
        [signature],
        { searchTransactionHistory: true },
      ]);
      const status = statuses.value[0];
      if (!status) {
        const height = await rpc("getBlockHeight", [{ commitment: "confirmed" }]);
        if (height > lastValidBlockHeight) {
          trace.emit(
            "solana.status.observed",
            {
              observer_source_id: observerSourceId,
              source_result: "not_found",
              duration_ns: elapsedNs(startedAt),
              signature,
              block_height: String(height),
            },
            { observer: true },
          );
          return { landing: "expired" };
        }
        break check;
      }
      const commitment = status.confirmationStatus ?? "processed";
      if (!best || COMMITMENT_RANK[commitment] > COMMITMENT_RANK[best]) {
        best = commitment;
        trace.emit(
          "solana.status.observed",
          {
            observer_source_id: observerSourceId,
            source_result: "found",
            duration_ns: elapsedNs(startedAt),
            signature,
            commitment,
            slot: String(status.slot),
            ...(status.confirmations === null || status.confirmations === undefined
              ? {}
              : { confirmations: String(status.confirmations) }),
          },
          { observer: true },
        );
        await trace.flush();
      }
      if (!execution && COMMITMENT_RANK[commitment] >= COMMITMENT_RANK.confirmed) {
        execution = await enrich(ctx, trace, signature, signed);
      }
      if (commitment === "finalized") return { landing: "finalized", execution };
    }
    await sleep(2_000);
  }
  return { landing: best ?? "incomplete", execution };
}

async function enrich(ctx, trace, signature, signed) {
  const { rpc, observerSourceId } = ctx;
  const tx = await rpc("getTransaction", [
    signature,
    { encoding: "json", commitment: "confirmed", maxSupportedTransactionVersion: 0 },
  ]);
  if (!tx?.meta) return undefined;
  const error = refineWithLogs(normalizeTransactionError(tx.meta.err, signed), tx.meta.logMessages);
  trace.emit(
    "solana.execution.enriched",
    {
      observer_source_id: observerSourceId,
      signature,
      slot: String(tx.slot),
      commitment: "confirmed",
      execution_result: tx.meta.err === null ? "success" : "failure",
      ...(tx.blockTime
        ? { block_time: new Date(tx.blockTime * 1000).toISOString().replace(".000Z", "Z") }
        : {}),
      fee_lamports: String(tx.meta.fee),
      ...(tx.meta.computeUnitsConsumed === undefined
        ? {}
        : { compute_units_consumed: String(tx.meta.computeUnitsConsumed) }),
      logs_present: Array.isArray(tx.meta.logMessages) && tx.meta.logMessages.length > 0,
      ...(error ? { error } : {}),
    },
    { observer: true },
  );
  await trace.flush();
  return tx.meta.err === null ? "success" : "failure";
}

async function pollStatus(rpc, signature, target, timeoutMs, intervalMs) {
  const deadline = Date.now() + timeoutMs;
  do {
    const status = (
      await rpc("getSignatureStatuses", [[signature], { searchTransactionHistory: true }])
    ).value[0];
    if (
      status &&
      COMMITMENT_RANK[status.confirmationStatus ?? "processed"] >= COMMITMENT_RANK[target]
    )
      return status;
    if (Date.now() + intervalMs > deadline) return undefined;
    await sleep(intervalMs);
  } while (Date.now() < deadline);
  return undefined;
}

async function waitForBlockHeight(rpc, target, timeoutMs) {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    if ((await rpc("getBlockHeight", [{ commitment: "confirmed" }])) >= target) return;
    await sleep(2_000);
  }
  throw new Error(`block height did not reach ${target} within ${timeoutMs} ms`);
}
