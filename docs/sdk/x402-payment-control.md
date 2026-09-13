# x402 payment control integration

**Audience:** application engineers operating a Solana-compatible x402 buyer.

**Purpose:** apply Landfall spend policies before a wallet signs an x402 payment,
send the signed header directly to the merchant, and retain a durable outcome
without giving Landfall custody of a wallet or a payment payload.

## What this integration does

Landfall is a policy and audit service, not a payment processor. The payment
path is:

```text
app → merchant (initial request) → HTTP 402 + PAYMENT-REQUIRED
app → Landfall /authorize → approved or denied
app wallet → creates PAYMENT-SIGNATURE
app → merchant (paid retry) → response
app → Landfall /settlements → settled or failed
```

The wallet key and `PAYMENT-SIGNATURE` remain in the application process. They
are not sent to Landfall, logged by Landfall, or displayed in the dashboard.

## Prerequisites

1. A running Landfall server and a project administrator token.
2. An enabled x402 policy matching the merchant's exact `(agent, network,
   asset, HTTPS origin)` and a small test limit.
3. A token containing only the `x402:pay` scope for the application.
4. An x402 merchant that returns a v2 `PAYMENT-REQUIRED` header.
5. A customer-owned wallet signer. For Solana, configure an official x402 SVM
   `exact` client in the wallet process.

Do not put a seed phrase, private key, signed transaction, or bearer token in
source code, telemetry, a Git commit, or a chat message.

## Configure a policy

Create the policy through Dashboard or the control-plane endpoint. The origin
is the scheme and host only — no path, query string, or credentials.

```json
{
  "agent_id": "pricing-agent",
  "network": "solana:5eykt4UsFv8P8NJdTREpY1vzqKqZKvdp",
  "asset": "USDC",
  "max_per_request_atomic": "100000",
  "max_per_day_atomic": "500000",
  "merchant_origins": ["https://merchant.example"],
  "enabled": true
}
```

`max_per_request_atomic` and `max_per_day_atomic` are integers in the asset's
smallest unit. They are not decimal display amounts.

## Application integration

The first request is intentionally unpaid. Select one requirement from the
merchant challenge, then have Landfall authorize that exact network, asset,
origin, and atomic amount before invoking the wallet.

```ts
import {
  createHttpX402Authorizer,
  createHttpX402ResourceClient,
  createHttpX402SettlementRecorder,
  executeAuthorizedX402Payment,
  parsePaymentRequiredHeader,
  preAuthorizeX402Requirement,
} from "@landfall/sdk";

const first = await fetch("https://merchant.example/premium-data");
if (first.status !== 402) throw new Error(`expected 402, got ${first.status}`);

const header = first.headers.get("PAYMENT-REQUIRED");
if (header === null) throw new Error("merchant did not send PAYMENT-REQUIRED");

const challenge = parsePaymentRequiredHeader(header);
const requirement = challenge.accepts[0]; // Replace with your explicit selector.
const authorization = preAuthorizeX402Requirement(
  challenge,
  requirement,
  process.env.LANDFALL_X402_POLICY_ID!,
  "pricing-agent",
  crypto.randomUUID(),
);

const policyFetch = async (url: string, init: RequestInit) => {
  const response = await fetch(url, init);
  return { status: response.status, json: () => response.json() };
};
const resourceFetch = async (url: string, init: RequestInit) => {
  const response = await fetch(url, init);
  return { ok: response.ok, status: response.status };
};

const outcome = await executeAuthorizedX402Payment(
  authorization,
  requirement,
  authorizer,
  walletOwnedSigner,
  createHttpX402ResourceClient({ url: "https://merchant.example/premium-data" }, resourceFetch),
  settlementRecorder,
);
```

Construct `authorizer` and `settlementRecorder` once at application startup:

```ts
const authorizer = createHttpX402Authorizer(
  process.env.LANDFALL_POLICY_URL!,
  process.env.LANDFALL_X402_TOKEN!,
  policyFetch,
);
const settlementRecorder = createHttpX402SettlementRecorder(
  process.env.LANDFALL_POLICY_URL!,
  process.env.LANDFALL_X402_TOKEN!,
  policyFetch,
);
```

`walletOwnedSigner` implements `X402PaymentSigner`. It must create the
official x402 SVM `exact` payload inside the wallet process. The optional
`createOfficialSvmExactSigner` helper wraps an already-configured official
`x402HTTPClient`; it deliberately does not accept a key, seed phrase, or key
file.

## Outcomes and recovery

| Outcome | Meaning | Application action |
| --- | --- | --- |
| `denied` | Policy rejected the merchant, network, asset, amount, or daily cap | Do not call the wallet or merchant; show the reason code. |
| `settled` | Merchant returned a successful paid response | Use the returned resource response. |
| `already-settled` | Same idempotency key was settled earlier | Do not call the wallet again; retrieve/reuse the original business result if applicable. |
| `already-failed` | Same idempotency key previously failed | Investigate and intentionally choose a new idempotency key only for a new business operation. |
| thrown error | Wallet, network, or merchant call failed | Landfall records `failed`; retain the error in your application logs without storing the payment payload. |

An idempotency key must represent one business payment intent. Reusing it after
`settled` is blocked specifically to prevent a second wallet signing attempt.

## Verify the audit trail

Open Dashboard → **x402 payments**, provide the project ID and a project
administrator token, and confirm the record. The dashboard shows merchant
origin, amount, decision, reason, and optional receipt. It never shows the
payment signature.

The equivalent read API is:

```text
GET /v1/control/projects/{project_id}/x402/audit?limit=50
Authorization: Bearer <project-admin-token>
```

The OpenAPI contract at `GET /openapi.json` is the authoritative API schema.

## Testnet acceptance run

Use the local-only checklist at `.landfall/x402-testnet-inputs.md`, set its
variables in a terminal, and run:

```bash
pnpm test:x402-testnet
```

The harness fails before signing if required inputs are missing or the merchant
does not return `402` with `PAYMENT-REQUIRED`. A successful live run prints an
`audit_id` and must produce a `settled` dashboard entry.

The official x402 protocol defines the HTTP `PAYMENT-REQUIRED` and
`PAYMENT-SIGNATURE` flow and requires merchant-side verification before a
resource is served. See the [x402 v2 specification](https://github.com/x402-foundation/x402/blob/main/specs/x402-specification-v2.md).
