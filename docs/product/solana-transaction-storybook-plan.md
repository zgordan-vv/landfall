# Solana transaction storybook — post-launch education plan

## Purpose

Create a short Russian-language illustrated guide for non-blockchain founders,
product managers, and support engineers evaluating Landfall. It must explain
the minimum Solana vocabulary needed to understand a Landfall trace, without
teaching readers to write smart contracts or manage wallets.

The working format is a "cartoon book": one continuing transaction, visual
characters, short scenes, and a plain-language explanation after each scene.
The production version is a web guide in the documentation site; printable PDF
and short social-media excerpts are derived later from the same source.

## Audience and user pains

| Reader | Pain before the guide | What the guide changes |
|---|---|---|
| Technical founder new to Solana | A wallet says a transaction was sent, but there is no clear answer whether it landed or why it failed. | Separates signing, RPC acknowledgement, network propagation, inclusion, and confirmation. |
| Product/support lead | Cannot explain a missing payment to a user without escalating every case to an engineer. | Gives honest wording for `sent`, `observed`, `confirmed`, `expired`, and `unknown`. |
| Backend engineer from Web2 | Treats an RPC timeout as proof that a transaction failed, or treats an accepted response as proof that it succeeded. | Shows why independent observation and evidence history matter. |
| Buyer evaluating Landfall | Sees telemetry screens but cannot connect them to an economic or support problem. | Connects each trace stage to support tickets, lost conversion, duplicate-payment risk, and provider incidents. |

The guide must not claim that Landfall guarantees transaction delivery, controls a
wallet, or proves an unobserved cause. Its central promise is narrower and
useful: Landfall preserves the evidence needed to explain what was observed.

## Narrative and learning outcomes

The protagonists are **Mila**, a product engineer whose checkout needs to send
a Solana payment, and **Leo**, a customer who expects a clear answer. The
recurring antagonist is **Captain "Probably"**: he turns incomplete evidence
into confident but wrong explanations. Landfall is not a hero with magical
network access; it is Mila's case file and timeline.

After reading, a user should be able to:

1. distinguish the global Solana network from an RPC provider's view of it;
2. describe a wallet signature, an RPC submission response, block inclusion,
   and confirmation as different events;
3. understand why an application may use one RPC to submit and Landfall may
   observe through another;
4. read a Landfall trace and its certainty labels without mistaking missing
   evidence for a confirmed conclusion;
5. identify when a support issue should be investigated in Landfall rather than
   answered from a wallet notification or a single provider dashboard.

## Chapter outline

### 1. One city, many windows

Solana is presented as one city where transactions travel. RPC providers are
different windows onto the same city. This resolves the most common newcomer
confusion: a paid RPC is not a private blockchain and a user does not need to
choose one merely to pay. Include a side-by-side diagram of application → RPC
A → Solana and Landfall → RPC B → Solana.

### 2. Leo clicks “Pay”

Show the business action (`checkout payment`) before the blockchain operation.
Explain why support cares about the business action, not only a transaction
signature. The pain addressed: one customer payment can involve retries or
replacements, and a team otherwise loses the connection between the receipt and
the technical evidence.

### 3. The wallet signs, but does not deliver a receipt

Explain that signing authorizes a specific transaction; it is not confirmation
that the network accepted it. Keep private keys off-page except for an explicit
boundary: neither Landfall nor an RPC provider receives the user's seed phrase.

### 4. The first window says “accepted”

The submitting RPC accepts the request, then Captain Probably declares victory.
Mila stops him: an RPC acknowledgement is useful evidence, not final proof of
on-chain inclusion. Explain transport timeout and retry in a panel without
assuming either success or failure.

### 5. A second window watches the city

Landfall's observer asks its configured route for signature status and block
height. Explain the benefit of independence: it can reveal a provider-specific
outage or a gap in evidence. Also explain the limit: two RPCs can still share
network conditions, so disagreement is investigated rather than magically
resolved.

### 6. Blocks, confirmation, and the clock that is not a clock

Introduce a block as a page in the city ledger and block height as its page
number. Explain confirmation as the network progressing beyond the transaction,
not as a fixed countdown. Show `lastValidBlockHeight` and an honest “expired
without observed inclusion” outcome.

### 7. The Landfall case file

Map the comic scenes to a real trace timeline: intent, sign, submit, observed
status, enrichment, diagnostics, and recommendation. Show three evidence labels:
**Confirmed**, **Probable**, and **Unknown**. The chapter must explicitly show
an Unknown result as a correct answer when evidence is insufficient.

### 8. What the team does next

Turn findings into actions: add a second observer, adjust retry behavior,
investigate provider throttling, or tell Leo the precise status. End with a
small decision table linking a trace state to the next owner: support, backend,
or RPC provider.

## Delivery slices and acceptance criteria

1. Write a plain-text pilot script and validate terminology against the product
   PRD and system-design certainty rules.
2. Create an accessible diagram style and storyboard; every illustration needs
   alt text and a text-only equivalent.
3. Publish the web guide with links to the real dashboard trace, SDK guide, and
   diagnostics catalog.
4. Test it with three readers who know Web2 but not Solana. Ask them to explain
   the difference between “signed”, “RPC accepted”, and “confirmed” in their
   own words.
5. Revise any chapter where two readers confuse an observation with a guarantee.

The guide is complete only if readers can explain why they would use Landfall
to resolve a real support or reliability problem. A glossary alone, a list of
RPC methods, or decorative illustrations do not meet this goal.
