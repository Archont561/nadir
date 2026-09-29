---
type: Cost Analysis
title: "Billing — Stripe, Credits, and Cost Envelope"
description: "Hybrid billing (subscription + credits), plan and credit economics, Stripe checkout + webhook idempotency, refunds, reconciliation, the MVP cost envelope, and the scaling tiers."
purpose: Pricing, billing flows, and unit economics for the SaaS
last_updated: 2026-09-30
status: stable
related:
  - data-model.md
  - deployment-topology.md
  - infrastructure.md
---

# Billing — Stripe, Credits, and Cost Envelope

## Hybrid Model: Subscription + Credits

```
┌─────────────────────────────────────────────────────┐
│                   SUBSCRIPTION                      │
│  Free:  $0/mo   → 500 one-time credits              │
│  Pro:   €29/mo  → 5,000 credits/mo, all exports     │
│  Team:  €99/mo  → 25,000 credits/mo, priority queue │
└──────────────────────────┬──────────────────────────┘
                           ▼
┌─────────────────────────────────────────────────────┐
│                   CREDIT SPEND                      │
│  1 credit   = 1 image (standard pipeline)           │
│  2 credits  = 1 image (survey / high-res)           │
│  0.5 credit = 1 image (fast, <100 imgs)             │
│                                                     │
│  Deducted when the workflow completes.              │
│  Refunded if the workflow fails or is cancelled.    │
└─────────────────────────────────────────────────────┘
```

**Why credits, not pure metering**: users understand "500 images" better
than "$0.058 per GPU-second". Credits also decouple pricing from GPU cost —
if RunPod prices change, adjust the credit-to-GPU ratio, not the price list.

| Plan | Monthly credits | Price | Effective per-image |
|---|---|---|---|
| Free | 500 (one-time) | $0 | — |
| Pro | 5,000/mo | €29/mo | €0.006 |
| Team | 25,000/mo | €99/mo | €0.004 |
| Top-up (1,000) | 1,000 | €10 | €0.01 |

## Why Stripe (not Lemon Squeezy / Paddle)

- **Usage meters are native** — Stripe Meters aggregate per-image usage
  from the credit ledger for invoicing
- **Lower fees** — ~1.5–2.5% + €0.25 (EU cards) + ~0.7% billing vs ~5% MoR.
  At €1k/mo revenue: ~€22 vs ~€55
- **EU VAT** via Stripe Tax add-on (register for VAT in Poland; automatic
  calculation). Revisit a merchant-of-record if compliance becomes a burden

## Stripe Flows (Gateway Side)

All Stripe calls live in `$lib/server/services/stripe.ts` (server-only):

**Top-up checkout** — `stripe.checkout.sessions.create({ mode: 'payment',
line_items: [price_data: { currency: 'eur', unit_amount: 1000,
product_data: { name: '1,000 NadrScan Credits' } }], customer,
success_url, cancel_url })` → redirect to `session.url`.

**Subscription upgrade** — same with `mode: 'subscription'` and the plan's
price ID (`STRIPE_PRICE_PRO` / `STRIPE_PRICE_TEAM`).

**Webhook** — `POST /api/webhooks/stripe`, signature-verified with
`STRIPE_WEBHOOK_SECRET`:

| Event | Action | Idempotency |
|---|---|---|
| `checkout.session.completed` | grant credits (`purchase`, ref `pi_xxx`) or activate plan + monthly grant (`subscription_grant`, ref `sub_xxx`) | unique `(reference_id, type)` ledger index |
| `invoice.paid` | monthly `subscription_grant` | ref `sub_xxx` + invoice id |
| `customer.subscription.deleted` | downgrade plan to free | upsert |
| `charge.refunded` | claw back top-up credits (adjustment) | ref refund id |

Stripe retries webhooks for days — every handler must be idempotent, which
the ledger's unique index guarantees structurally.

## Refunds on Failure

The pipeline's undo chain refunds processing credits when a workflow fails
or is cancelled (`refund`, reference `proj_xxx`). User errors (bad imagery)
refund in full; infra errors refund in full + retry for free; the ledger
description records which.

## Reconciliation

Daily job (V1): recompute `Σ credit_transactions` per user and repair
`users.credits` drift; sync Stripe Meter values from the ledger; alert on
any negative balance or grant without a matching Stripe object.

## MVP Cost Envelope

~$60/month at 50 jobs/month — see [context.md](context.md) for the
line-item table (Clerk $0, Neon $0, R2 ~$7.50, RunPod ~$29, Stripe ~$18,
Fly.io ~$5).

## Scaling Tiers

| Tier | Users | Jobs/mo | Revenue | Infra cost | Margin |
|---|---|---|---|---|---|
| **Starter** | 0–500 | 0–200 | $0–5k | $68–200 | 60–80% |
| **Growth** | 500–5,000 | 200–2,000 | $5k–50k | $200–2k | 80–90% |
| **Scale** | 5k–50k | 2k–20k | $50k–500k | $2k–20k | 85–92% |
| **Enterprise** | 50k+ | 20k+ | $500k+ | $20k+ | 90%+ |

### Scaling decision tree

```
Gateway CPU > 80%?            → add gateway instances (Fly autoscale)
SSE connections > 500?        → multi-region gateways
Neon cold start > 500ms?      → Neon Launch plan ($19)
RunPod queue depth > 10?      → raise max_workers
RunPod cold start > 30s?      → min_workers: 1–2 (Active mode)
GPU bill > $5k/mo?            → self-host GPUs (Hetzner ≈ €1,500/mo vs $5k)
R2 storage > 50TB?            → lifecycle rules (raw → delete after 30–90d)
Stripe fees > $3k/mo?         → volume discount or MoR
```

## See Also

- [Data model](data-model.md) — the credit ledger that backs all of this
- [Auth](auth.md) — where the Stripe customer is created
- [Deployment topology](deployment-topology.md) — what the money buys
- [Infrastructure](infrastructure.md) — per-service pricing notes
