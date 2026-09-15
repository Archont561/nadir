---
title: Hosting Comparison
purpose: Railway vs Render vs Vercel vs Fly.io vs VPS for photogrammetry workloads
last_updated: 2025-02-23
status: current
related:
  - docker-strategy.md
  - demo-plan.md
  - ../decisions/004-inverted-container-no-dind.md
---

# Hosting Comparison

## TL;DR

Photogrammetry is CPU/RAM/disk-heavy. A 100-image dataset can take 30–60
minutes and 16GB RAM. This rules out serverless platforms (Vercel, Netlify)
for the worker. **Railway** or a **cheap VPS** (Hetzner) are the best options
for the worker. **Vercel** is fine for the UI only.

## Platform Comparison

| Feature | Railway | Render | Fly.io | Vercel | Hetzner VPS |
|---|---|---|---|---|---|
| **Docker support** | ✅ | ✅ | ✅ | ❌ | ✅ (full) |
| **Max RAM** | 8 GB | 2 GB (free) / 24 GB (paid) | 8 GB (free) / 64 GB (paid) | 1 GB | 32–256 GB |
| **Max CPU** | 8 vCPU | 2 (free) / 8 (paid) | 4 (free) / 16 (paid) | N/A | 8–64 cores |
| **GPU** | ❌ | ❌ | ✅ (paid) | ❌ | ✅ (paid) |
| **Persistent disk** | ✅ (volumes) | ✅ (paid) | ✅ (volumes) | ❌ | ✅ (full) |
| **Long-running tasks** | ✅ (no timeout) | ✅ (no timeout) | ✅ | ❌ (10s limit) | ✅ |
| **Docker-in-Docker** | ❌ | ❌ | ❌ | ❌ | ✅ |
| **Cold start** | ~5s | ~30s (free) | ~1s | ~0s | N/A |
| **Free tier** | $5 credit | 2GB RAM, spins down | 3 VMs, limited | Generous | None |
| **Cost (8GB worker)** | ~$20/mo | ~$25/mo | ~$20/mo | N/A | ~$15/mo |
| **Always on** | ✅ | ❌ (sleeps after 15min) | ⚠️ (limited) | ✅ | ✅ |
| **Best for** | Worker + API | Worker (paid) | Worker + GPU | UI only | Full control |

## Why Vercel Doesn't Work for Workers

Vercel is a serverless platform with:
- 10-second function timeout (photogrammetry takes hours)
- 1 GB memory limit (photogrammetry needs 16–64 GB)
- No persistent disk (photogrammetry needs 50–500 GB)
- No Docker support (can't run COLMAP/OpenMVS)
- No long-running processes

Vercel is excellent for the **Next.js UI** but cannot run the photogrammetry
worker.

## Recommended Architecture by Phase

### V0.1 Demo (single machine)

```
┌─────────────┐     ┌─────────────────┐
│   Vercel    │────▶│    Railway      │
│   Next.js   │     │   8GB instance  │
│   UI + BFF  │◀────│   Nadir worker  │
│   (free)    │     │   + ODM engines │
└─────────────┘     └────────┬────────┘
                             │
                      ┌──────▼──────┐
                      │ Cloudflare  │
                      │ R2 (10GB)   │
                      │ Storage     │
                      └─────────────┘

Monthly cost: ~$20 (Railway) + $0 (Vercel) + $0 (R2) = ~$20
```

### V1.0 Production

```
┌─────────────┐     ┌─────────────────┐
│   Vercel    │────▶│    Railway      │
│   Next.js   │     │   API + BFF     │
│   UI        │     │   (2GB, $5)     │
└─────────────┘     └────────┬────────┘
                             │
                    ┌────────▼────────┐
                    │  Hetzner VPS    │
                    │  32GB RAM       │
                    │  8 cores        │
                    │  Nadir workers  │
                    │  ($15/mo)       │
                    └────────┬────────┘
                             │
                      ┌──────▼──────┐
                      │ Cloudflare  │
                      │ R2 / S3     │
                      │ Storage     │
                      └─────────────┘

Monthly cost: ~$20
```

### V2.0 Distributed

```
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│   Vercel    │────▶│   Railway   │────▶│   Fly.io    │
│   UI        │     │   Scheduler │     │   GPU worker│
│             │     │   + API     │     │   (COLMAP)  │
└─────────────┘     └──────┬──────┘     └─────────────┘
                           │
                    ┌──────▼──────┐     ┌─────────────┐
                    │   Railway   │     │   Railway   │
                    │   Carto     │     │   Surface   │
                    │   worker    │     │   worker    │
                    └─────────────┘     └─────────────┘
                           │
                    ┌──────▼──────┐
                    │  S3 / R2    │
                    │  Storage    │
                    └─────────────┘

Monthly cost: ~$80–150
```

## Photogrammetry Resource Requirements

| Dataset Size | Images | RAM | CPU | Disk | Time | GPU |
|---|---|---|---|---|---|---|
| Tiny demo | 20–50 | 4 GB | 2 cores | 5 GB | 10–30 min | No |
| Small | 100–300 | 8 GB | 4 cores | 20 GB | 1–3 hours | Optional |
| Medium | 500–1,000 | 16 GB | 8 cores | 50 GB | 3–8 hours | Recommended |
| Large | 2,000–5,000 | 32 GB | 16 cores | 200 GB | 8–24 hours | Yes |
| Huge | 10,000+ | 64+ GB | 32+ cores | 500+ GB | 1–3 days | Required |

### Per-stage resource profile

| Stage | CPU | RAM | GPU | Disk I/O |
|---|---|---|---|---|
| Feature extraction | High | Medium | Optional | Read-heavy |
| Matching | High | High | Optional | Read/write |
| SfM | Medium | High | No | Write |
| Dense MVS | High | Very High | Optional | Write-heavy |
| Mesh | Medium | High | No | Read/write |
| DSM/DTM | Medium | Medium | No | Write |
| Orthomosaic | Medium | Medium | No | Read/write |

## Demo Hosting Cost Breakdown

| Service | Tier | Monthly Cost |
|---|---|---|
| Railway (worker, 8GB) | Pro | ~$20 |
| Railway (API, 1GB) | Pro | ~$5 |
| Vercel (UI) | Free | $0 |
| Cloudflare R2 (storage, 10GB) | Free | $0 |
| Domain (.dev or .io) | Annual | ~$1/mo |
| **Total** | | **~$25/month** |

## Decision Matrix

| If you need... | Use... |
|---|---|
| Quick demo, minimal cost | Railway (worker) + Vercel (UI) |
| Full control, lowest cost | Hetzner VPS |
| GPU acceleration | Fly.io (GPU machines) |
| Zero DevOps | Railway |
| Multi-region | Fly.io |
| Enterprise / compliance | AWS/GCP/Azure + Kubernetes |

## See Also

- [Docker strategy](./docker-strategy.md) — container model for each platform
- [Demo plan](./demo-plan.md) — 4-week demo using Railway + Vercel
- [ADR-004: Inverted container](../decisions/004-inverted-container-no-dind.md)
