# Nadir Knowledge Bundle Update Log

## 2026-09-30
* **Creation**: Added the `saas/` section (13 concepts) — the NadrScan photogrammetry SaaS, ported from the external NadirScan knowledge bundle (`Archont561/NadirScan`, OKF v0.2, fetched 2026-09-30) and adapted from a Lintel BFF gateway to SvelteKit.
* **Creation**: [ADR-009](decisions/009-sveltekit-gateway-for-nadrscan.md) — the NadrScan gateway will be `apps/nadirscan` built with SvelteKit; records the Lintel-primitive → SvelteKit mapping and the monorepo layout.
* **Update**: `context.md` — tech stack, key decisions, current status, naming conventions, and file navigation now include the SaaS.
* **Update**: `index.md` and `legacy-navigation.md` — new SaaS section and ADR-009 entries.
* **Deprecation**: `deployment/demo-plan.md` demo stack (Astro + WebcoreUI) superseded by ADR-009; the Clerk + Postgres/Drizzle + Docker-worker parts carry over into `saas/`.

## 2026-09-17
* **Migration**: Updated the bundle to conform to Open Knowledge Format (OKF) v0.2.
* **Metadata**: Added concept types and descriptions, and normalized lifecycle status values.
* **Navigation**: Added the reserved [bundle index](/index.md) and converted the project context to a concept.
