---
type: Authentication
title: "Auth — Clerk in SvelteKit"
description: "JWT verification in hooks.server.ts, the Svix-verified Clerk webhook, the Clerk → Neon → Stripe → Resend provisioning chain with three layers of idempotency, and client-side auth components."
purpose: Authentication and user-lifecycle wiring for the SaaS
last_updated: 2026-09-30
status: stable
related:
  - data-model.md
  - billing.md
  - context.md
---

# Auth — Clerk in SvelteKit

## Why Clerk

- **Free tier** — 10k MAU covers the entire MVP period
- **Clerk-hosted UI** — sign-in/sign-up pages we never build or style
- **JWT sessions** — the gateway verifies tokens locally via JWKS; no
  session table, no auth round-trip per request
- **Webhooks** — user lifecycle events (created/updated/deleted) drive
  provisioning

## Server-Side: hooks.server.ts

Every request passes through one `handle` hook: verify the Clerk session
JWT against Clerk's JWKS endpoint, load the user (L1-cached 5 min), attach
to `event.locals`.

```typescript
// apps/nadirscan/src/hooks.server.ts
import { jose } from '$lib/server/auth/jwt'; // createRemoteJWKSet(clerkIssuer + '/.well-known/jwkeys.json')
import { store } from '$lib/server/db';

export const handle: Handle = async ({ event, resolve }) => {
	const token = event.cookies.get('__session');
	if (token) {
		try {
			const claims = await jose.verifyJwt(token);           // cacheable 5m
			event.locals.user = await store.getUser(claims.sub);  // L1 cache
		} catch {
			event.locals.user = null; // expired/invalid → anonymous
		}
	}
	return resolve(event);
};
```

Route protection is a guard helper used in `load()` and actions:

```typescript
// +page.server.ts
export const load = async ({ locals }) => {
	requireUser(locals);            // redirects to CLERK_SIGN_IN_URL if absent
	return { projects: await store.listProjects(locals.user!.id) };
};
```

JWT verification uses `jose` (`createRemoteJWKSet` + `jwtVerify` with the
`iss` = `CLERK_ISSUER_URL` check). No Clerk SDK needed server-side.

## The Provisioning Webhook

```
POST /api/webhooks/clerk
  │ Svix signature verification (CLERK_WEBHOOK_SECRET)
  │ event.type === 'user.created'
  ▼
Provisioning chain (idempotent at every layer):
  1. Neon     → INSERT users row (ON CONFLICT (id) DO NOTHING)
               + grant 500 signup credits (referenceId 'signup-bonus')
  2. Stripe   → create customer (metadata.clerkId) → save stripe_customer_id
  3. Resend   → send welcome email (queued, best-effort)
```

Three layers of idempotency handle webhook retries and races:

| Layer | Mechanism |
|---|---|
| Neon insert | `ON CONFLICT (id) DO NOTHING` / upsert |
| Credit grant | unique `(reference_id, type)` index on `credit_transactions` |
| Stripe customer | lookup by `metadata.clerkId` before create |

`user.deleted` → soft delete (`role = 'deleted'`), never row removal
(foreign keys from projects/transactions). `user.updated` → refresh
email/name/image.

## Client-Side

Clerk's browser package mounts inside a Svelte component (ClerkProvider +
`<SignIn />` / `<UserButton />` are web components — no React needed):

```svelte
<!-- src/routes/sign-in/+page.svelte -->
<script>
	import { ClerkProvider, SignIn } from '@clerk/svelte';
	// or mount Clerk's web components directly
</script>

<ClerkProvider publishableKey={env.CLERK_PUBLISHABLE_KEY}>
	<SignIn />
</ClerkProvider>
```

Auth state reaches server code via the `__session` cookie (HttpOnly) —
client components never pass tokens by hand. The sign-in redirect target is
`CLERK_SIGN_IN_URL`.

## Demo Mode

Without `CLERK_*` env vars, `hooks.server.ts` falls back to a fixed demo
user (`user_demo`) so the entire product is walkable without auth setup.

## Onboarding Timeline

```
t=0     user signs up (Clerk-hosted page)
t+1s    Clerk webhook fires (user.created)
t+2s    Neon row + 500 credits granted
t+3s    Stripe customer created
t+5s    welcome email queued
t+30s   user can upload (quota check passes)
```

## See Also

- [Data model](data-model.md) — users table + credit ledger
- [Billing](billing.md) — what the Stripe customer is used for
- [Context](context.md) — the five core decisions
