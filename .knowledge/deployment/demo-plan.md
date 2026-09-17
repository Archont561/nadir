---
type: Deployment Guide
title: Demo Plan
description: "4-week demo — Astro + WebcoreUI + Clerk + Postgres + Drizzle + Inngest + Docker worker"
purpose: 4-week demo — Astro + WebcoreUI + Clerk + Postgres + Drizzle + Inngest + Docker worker
last_updated: 2025-06-20
status: stable
related:
  - hosting-comparison.md
  - docker-strategy.md
  - ../roadmap/v0-mvp.md
  - ../decisions/004-inverted-container-no-dind.md
---

# Demo Plan

## TL;DR

Build a working Nadir demo in 4 weeks using a **modular monolith with
durable background jobs**. Stack: **Astro** (UI + BFF) + **WebcoreUI**
(component library) + **Clerk** (auth) + **PostgreSQL** (Docker locally,
managed in production) + **Drizzle ORM** (type-safe queries + migrations) +
**Inngest** (async orchestration) + **Docker worker** (Nadir Rust CLI +
engines). Cost: ~$30/month.

## The Architectural Contract

The demo follows the **default architecture**: one Astro application, one
database, one Inngest integration, one worker image. No microservices. No
Kubernetes. No Redis. No custom queue. Add complexity only when the
product concretely requires it.

## What the Demo Proves

The demo proves the **platform concept**, not the photogrammetry:

1. Real authentication (Clerk)
2. Clean developer UX (upload → process → result)
3. Durable async orchestration (Inngest)
4. Heavy compute in isolated worker (Docker)
5. Real-time progress with job state in Postgres
6. Georeferenced output on a map

It does NOT need to prove: step-scoped engines, adaptive pipelines, QC
gates, distributed workers, or multi-language SDKs. Those come later.

## The Demo Architecture

```
                         ┌──────────────────┐
                         │     Browser      │
                         └────────┬─────────┘
                                  │
                                  ▼
                  ┌─────────────────────────────┐
                  │ Astro                       │
                  │                             │
                  │ WebcoreUI components        │
                  │ Pages: /, /projects, /jobs  │
                  │ BFF: /api/*                 │
                  │ Auth: @clerk/astro          │
                  │ ORM: Drizzle                │
                  └─────────────┬───────────────┘
                                │
                ┌───────────────┼────────────────┐
                │               │                │
                ▼               ▼                ▼
             Clerk          PostgreSQL        Inngest
           identity       (via Drizzle)         │
                                                 │
                                                 ▼
                                        ┌─────────────────┐
                                        │  Docker Worker  │
                                        │                 │
                                        │  Nadir Rust CLI │
                                        │  + COLMAP       │
                                        │  + OpenMVS      │
                                        │  + GDAL/PROJ    │
                                        │  (via Pixi)     │
                                        │  + Drizzle      │
                                        └─────────────────┘
                                                 │
                                                 ▼
                                        ┌─────────────────┐
                                        │ Cloudflare R2   │
                                        │ Artifact store  │
                                        └─────────────────┘
```

## The Five Conceptual Layers

```
UI (Astro pages + WebcoreUI components)
 ↓
BFF (Astro server routes / src/pages/api/)
 ↓
Domain logic (src/lib/{projects,jobs,users}.ts)
 ↓
Drizzle ORM (src/db/index.ts + src/db/schema.ts)
 ↓
PostgreSQL

Async path:
BFF
 ↓
Inngest event (project.process.requested)
 ↓
Docker Worker (Nadir CLI + Drizzle for status updates)
 ↓
PostgreSQL (job status update)
```

## Tech Stack Summary

| Component | Technology | Purpose |
|---|---|---|
| UI framework | Astro 5.x | Pages, SSR, BFF |
| Component library | WebcoreUI | Buttons, forms, cards, dialogs |
| Auth | `@clerk/astro` | Identity, sessions, OAuth |
| Database | PostgreSQL 18 | Application state, job records |
| ORM | Drizzle (`drizzle-orm` + `pg`) | Type-safe queries, migrations |
| Migrations | Drizzle Kit | Schema push / generate / migrate |
| Local DB | Docker Compose | `postgres:18` container |
| Async orchestration | Inngest | Events, retries, workflows |
| Heavy compute | Docker worker | Nadir Rust CLI + engines |
| Storage | Cloudflare R2 | Images + artifacts |
| Language | TypeScript | Type safety everywhere |
| Package manager | pnpm | Fast, disk-efficient |

## What to Cut

| Feature | Demo? | Why |
|---|---|---|
| Step-scoped engines | ❌ | Nadir CLI wraps ODM as engine for demo |
| Adaptive pipeline | ❌ | Fixed pipeline for demo |
| QC gates | ❌ | Check worker exit code |
| Content-addressed cache | ❌ | No caching between runs |
| Provenance UI | ❌ | Skip for demo |
| Multi-engine abstraction | ❌ | One pipeline |
| Billing | ❌ | Clerk auth only |
| Large dataset support | ❌ | Limit to 200 images |
| 3D mesh viewer | ⚠️ | Nice-to-have |
| Map preview of orthomosaic | ✅ | The "wow" moment |
| Real-time progress | ✅ | Makes demo feel alive |
| Download results | ✅ | The whole point |
| Auth | ✅ | Clerk, minimal setup |

## The 4-Week Plan

### Week 1: Worker + Nadir CLI Integration (5–7 days)

**Goal:** Nadir Rust CLI runs in Docker, produces outputs, uploads to R2.

- [ ] Rust CLI: `nadir process ./images --output ./outputs`
- [ ] Docker image: Pixi + COLMAP + OpenMVS + GDAL + Nadir binary
- [ ] Inngest function that:
  - [ ] Downloads images from R2
  - [ ] Spawns Nadir CLI subprocess
  - [ ] Streams stdout progress
  - [ ] Uploads outputs to R2
  - [ ] Updates job status in Postgres via Drizzle
- [ ] Test locally with 20–50 drone images

**Worker Inngest function:**

```typescript
// worker/src/functions/process-project.ts
import { inngest } from '../inngest';
import { spawn } from 'child_process';
import { downloadFromR2, uploadToR2 } from '../r2';
import { db } from '../db';
import { jobs } from '../db/schema';
import { eq, sql } from 'drizzle-orm';

export const processProject = inngest.createFunction(
  {
    id: 'process-project',
    retries: 2,
    concurrency: { limit: 2 },
  },
  { event: 'project.process.requested' },
  async ({ event, step }) => {
    const { jobId, projectId, imageKeys } = event.data;

    // Step 1: Mark job as running
    await step.run('mark-running', async () => {
      await db
        .update(jobs)
        .set({ status: 'running', startedAt: new Date() })
        .where(eq(jobs.id, jobId));
    });

    // Step 2: Download images
    const workDir = `/var/nadir/workspace/${jobId}`;
    await step.run('download-images', async () => {
      await downloadFromR2(imageKeys, `${workDir}/images`);
    });

    // Step 3: Run Nadir CLI
    await step.run('nadir-process', async () => {
      await new Promise<void>((resolve, reject) => {
        const child = spawn('nadir', [
          'process',
          `${workDir}/images`,
          '--output', `${workDir}/outputs`,
          '--quality', 'standard',
          '--json',
        ]);

        child.stdout.on('data', async (chunk: Buffer) => {
          const lines = chunk.toString().split('\n').filter(Boolean);
          for (const line of lines) {
            try {
              const parsed = JSON.parse(line);
              if (parsed.type === 'progress') {
                await db
                  .update(jobs)
                  .set({
                    output: sql`jsonb_set(
                      COALESCE(${jobs.output}, '{}'::jsonb),
                      '{progress}',
                      ${JSON.stringify(parsed.progress)}::jsonb
                    )`,
                  })
                  .where(eq(jobs.id, jobId));
              }
            } catch { /* not JSON */ }
          }
        });

        child.on('exit', (code) => {
          code === 0 ? resolve() : reject(new Error(`exit ${code}`));
        });
      });
    });

    // Step 4: Upload outputs
    const outputKeys = await step.run('upload-outputs', async () => {
      return await uploadToR2(`${workDir}/outputs`, `projects/${projectId}/outputs`);
    });

    // Step 5: Mark completed
    await step.run('mark-completed', async () => {
      await db
        .update(jobs)
        .set({
          status: 'completed',
          completedAt: new Date(),
          output: sql`jsonb_set(
            COALESCE(${jobs.output}, '{}'::jsonb),
            '{artifacts}',
            ${JSON.stringify(outputKeys)}::jsonb
          )`,
        })
        .where(eq(jobs.id, jobId));
    });

    return { jobId, artifacts: outputKeys };
  }
);
```

### Week 2: Astro BFF + Database + Clerk (4–5 days)

**Goal:** Auth, project creation, job submission, status endpoint.

- [ ] `pnpm create astro@latest app`
- [ ] Install `@clerk/astro`, `webcoreui`, `drizzle-orm`, `pg`, `inngest`
- [ ] Install dev: `drizzle-kit`, `@types/pg`
- [ ] Configure Clerk in `astro.config.mjs` + `src/middleware.ts`
- [ ] Docker Compose for Postgres
- [ ] Drizzle schema: `src/db/schema.ts` (users, projects, jobs)
- [ ] Drizzle connection: `src/db/index.ts`
- [ ] Drizzle Kit config: `drizzle.config.ts`
- [ ] Push schema: `npx drizzle-kit push`
- [ ] Domain modules: `src/lib/{users,projects,jobs}.ts` (using Drizzle queries)
- [ ] BFF endpoints:
  - `POST /api/projects` — create project
  - `POST /api/projects/:id/upload-url` — presigned R2 URL
  - `POST /api/projects/:id/process` — emit Inngest event
  - `GET /api/projects/:id/status` — poll job status
  - `GET /api/inngest` — Inngest function serving

**Drizzle schema:**

```typescript
// app/src/db/schema.ts
import {
  pgTable,
  uuid,
  text,
  integer,
  timestamp,
  jsonb,
  index,
} from 'drizzle-orm/pg-core';

export const users = pgTable('users', {
  id: uuid('id').primaryKey().defaultRandom(),
  clerkUserId: text('clerk_user_id').unique().notNull(),
  email: text('email'),
  createdAt: timestamp('created_at', { withTimezone: true })
    .defaultNow()
    .notNull(),
  updatedAt: timestamp('updated_at', { withTimezone: true })
    .defaultNow()
    .notNull(),
});

export const projects = pgTable('projects', {
  id: uuid('id').primaryKey().defaultRandom(),
  userId: uuid('user_id')
    .notNull()
    .references(() => users.id, { onDelete: 'cascade' }),
  name: text('name').notNull(),
  imageCount: integer('image_count').default(0),
  createdAt: timestamp('created_at', { withTimezone: true })
    .defaultNow()
    .notNull(),
});

export const jobs = pgTable(
  'jobs',
  {
    id: uuid('id').primaryKey().defaultRandom(),
    userId: uuid('user_id')
      .notNull()
      .references(() => users.id, { onDelete: 'cascade' }),
    projectId: uuid('project_id')
      .notNull()
      .references(() => projects.id, { onDelete: 'cascade' }),
    type: text('type').notNull(),
    status: text('status', {
      enum: ['queued', 'running', 'completed', 'failed', 'cancelled'],
    }).notNull(),
    input: jsonb('input'),
    output: jsonb('output'),
    error: text('error'),
    createdAt: timestamp('created_at', { withTimezone: true })
      .defaultNow()
      .notNull(),
    startedAt: timestamp('started_at', { withTimezone: true }),
    completedAt: timestamp('completed_at', { withTimezone: true }),
  },
  (table) => [
    index('idx_jobs_user_id').on(table.userId),
    index('idx_jobs_project_id').on(table.projectId),
    index('idx_jobs_status').on(table.status),
  ]
);
```

**Drizzle connection:**

```typescript
// app/src/db/index.ts
import { drizzle } from 'drizzle-orm/node-postgres';
import { Pool } from 'pg';
import * as schema from './schema';

const pool = new Pool({
  connectionString: import.meta.env.DATABASE_URL,
});

export const db = drizzle({ client: pool, schema });
```

> **Important:** `DATABASE_URL` is server-side only. Astro only exposes
> `PUBLIC_*` variables to client code. **Never use `PUBLIC_DATABASE_URL`.**

**Drizzle Kit config:**

```typescript
// app/drizzle.config.ts
import 'dotenv/config';
import { defineConfig } from 'drizzle-kit';

export default defineConfig({
  schema: './src/db/schema.ts',
  out: './drizzle',
  dialect: 'postgresql',
  dbCredentials: {
    url: process.env.DATABASE_URL!,
  },
});
```

**Astro config:**

```javascript
// astro.config.mjs
import { defineConfig } from 'astro/config';
import clerk from '@clerk/astro';
import webcoreui from 'webcoreui/astro';
import node from '@astrojs/node';

export default defineConfig({
  integrations: [clerk(), webcoreui()],
  output: 'server',
  adapter: node({ mode: 'standalone' }),
});
```

**Middleware:**

```typescript
// src/middleware.ts
import { clerkMiddleware } from '@clerk/astro/server';

export const onRequest = clerkMiddleware();
```

**Project creation endpoint:**

```typescript
// src/pages/api/projects.ts
import type { APIRoute } from 'astro';
import { getOrCreateUser } from '../../lib/users';
import { createProject } from '../../lib/projects';
import { z } from 'zod';

const CreateProjectSchema = z.object({
  name: z.string().min(1).max(200),
});

export const POST: APIRoute = async ({ request, locals }) => {
  // 1. Authenticate
  const { userId: clerkUserId } = locals.auth();
  if (!clerkUserId) {
    return new Response('Unauthorized', { status: 401 });
  }

  // 2. Validate input
  const body = await request.json();
  const parsed = CreateProjectSchema.safeParse(body);
  if (!parsed.success) {
    return new Response(JSON.stringify(parsed.error), { status: 400 });
  }

  // 3. Get/create application user
  const user = await getOrCreateUser(clerkUserId);

  // 4. Create project (uses Drizzle internally)
  const project = await createProject(user.id, parsed.data.name);

  return new Response(JSON.stringify(project), {
    status: 201,
    headers: { 'Content-Type': 'application/json' },
  });
};
```

**Domain module example (Drizzle queries):**

```typescript
// src/lib/projects.ts
import { db } from '../db';
import { projects } from '../db/schema';
import { eq } from 'drizzle-orm';

export async function createProject(userId: string, name: string) {
  const [project] = await db
    .insert(projects)
    .values({ userId, name })
    .returning();
  return project;
}

export async function getProject(id: string) {
  const [project] = await db
    .select()
    .from(projects)
    .where(eq(projects.id, id));
  return project;
}

export async function listProjects(userId: string) {
  return await db
    .select()
    .from(projects)
    .where(eq(projects.userId, userId))
    .orderBy(projects.createdAt);
}
```

**Process endpoint (emits Inngest event):**

```typescript
// src/pages/api/projects/[id]/process.ts
import type { APIRoute } from 'astro';
import { getOrCreateUser } from '../../../../lib/users';
import { getProject } from '../../../../lib/projects';
import { createJob } from '../../../../lib/jobs';
import { inngest } from '../../../../lib/inngest';

export const POST: APIRoute = async ({ params, locals }) => {
  const { userId: clerkUserId } = locals.auth();
  if (!clerkUserId) {
    return new Response('Unauthorized', { status: 401 });
  }

  const user = await getOrCreateUser(clerkUserId);
  const project = await getProject(params.id!);

  // Authorization: user must own project
  if (project.userId !== user.id) {
    return new Response('Forbidden', { status: 403 });
  }

  // Create job record (Drizzle insert)
  const job = await createJob({
    userId: user.id,
    projectId: project.id,
    type: 'photogrammetry.mapping',
    input: { imageCount: project.imageCount },
  });

  // Emit Inngest event
  await inngest.send({
    name: 'project.process.requested',
    data: {
      jobId: job.id,
      projectId: project.id,
      userId: user.id,
      imageKeys: project.imageKeys,
    },
  });

  return new Response(JSON.stringify({ jobId: job.id }), {
    status: 202,
    headers: { 'Content-Type': 'application/json' },
  });
};
```

### Week 3: Astro UI + WebcoreUI (5–7 days)

**Goal:** Upload images, watch progress, download results.

- [ ] Landing page with Clerk sign-in
- [ ] `/dashboard` — list of projects
- [ ] `/projects/new` — create project + drag-drop upload
- [ ] `/projects/[id]` — status page with progress
- [ ] `/projects/[id]/results` — map preview + downloads
- [ ] Map component using `leaflet` + `geotiff.js`

**Dashboard page:**

```astro
---
// src/pages/dashboard.astro
import Layout from '../layouts/Layout.astro';
import { Card, Button } from 'webcoreui/astro';
import { SignedIn, SignedOut, RedirectToSignIn } from '@clerk/astro/components';
import { getOrCreateUser } from '../lib/users';
import { listProjects } from '../lib/projects';

const { userId: clerkUserId } = Astro.locals.auth();
if (!clerkUserId) return Astro.redirect('/sign-in');

const user = await getOrCreateUser(clerkUserId);
const projects = await listProjects(user.id);
---

<Layout title="Dashboard">
  <SignedIn>
    <div class="max-w-4xl mx-auto p-8">
      <div class="flex justify-between items-center mb-8">
        <h1 class="text-3xl font-bold">My Projects</h1>
        <Button href="/projects/new">New Project</Button>
      </div>

      <div class="grid gap-4">
        {projects.map((project) => (
          <Card>
            <a href={`/projects/${project.id}`}>
              <h2>{project.name}</h2>
              <p>{project.imageCount} images</p>
            </a>
          </Card>
        ))}
      </div>
    </div>
  </SignedIn>
  <SignedOut>
    <RedirectToSignIn />
  </SignedOut>
</Layout>
```

**Project status page (polls every 2s):**

```astro
---
// src/pages/projects/[id].astro
import Layout from '../../layouts/Layout.astro';
import { Card } from 'webcoreui/astro';
import { getOrCreateUser } from '../../lib/users';
import { getProject } from '../../lib/projects';
import { getLatestJob } from '../../lib/jobs';
import JobStatus from '../../components/JobStatus.astro';

const { userId: clerkUserId } = Astro.locals.auth();
if (!clerkUserId) return Astro.redirect('/sign-in');

const user = await getOrCreateUser(clerkUserId);
const project = await getProject(Astro.params.id!);

if (project.userId !== user.id) return new Response('Forbidden', { status: 403 });

const job = await getLatestJob(project.id);
---

<Layout title={project.name}>
  <div class="max-w-4xl mx-auto p-8">
    <h1 class="text-3xl font-bold mb-8">{project.name}</h1>

    {job && (
      <JobStatus client:only="preact" initialJob={job} jobId={job.id} />
    )}
  </div>
</Layout>
```

**Job status island (interactive polling):**

```tsx
// src/components/JobStatus.tsx
import { useEffect, useState } from 'preact/hooks';

export default function JobStatus({ initialJob, jobId }) {
  const [job, setJob] = useState(initialJob);

  useEffect(() => {
    if (job.status === 'completed' || job.status === 'failed') return;

    const interval = setInterval(async () => {
      const res = await fetch(`/api/jobs/${jobId}`);
      const updated = await res.json();
      setJob(updated);
      if (updated.status === 'completed' || updated.status === 'failed') {
        clearInterval(interval);
      }
    }, 2000);

    return () => clearInterval(interval);
  }, [jobId, job.status]);

  const progress = job.output?.progress?.overall ?? 0;
  const stage = job.output?.progress?.stage ?? 'queued';

  return (
    <div>
      <div class="mb-4">
        <p class="text-sm text-gray-600">Status: <strong>{job.status}</strong></p>
        {job.status === 'running' && (
          <>
            <p class="text-sm text-gray-600">Stage: {stage}</p>
            <div class="w-full bg-gray-200 rounded-full h-2 mt-2">
              <div
                class="bg-blue-600 h-2 rounded-full transition-all"
                style={`width: ${progress * 100}%`}
              />
            </div>
          </>
        )}
      </div>

      {job.status === 'completed' && (
        <a
          href={`/projects/${job.projectId}/results`}
          class="inline-block px-4 py-2 bg-green-600 text-white rounded"
        >
          View Results
        </a>
      )}
    </div>
  );
}
```

### Week 4: Deployment + Polish (5–7 days)

**Goal:** Everything works end-to-end on hosted infrastructure.

- [ ] Deploy Astro app to Railway (Node adapter)
- [ ] Deploy Docker worker to Railway (8GB RAM instance)
- [ ] Managed Postgres on Railway (or Neon)
- [ ] Run `npx drizzle-kit migrate` against production DB
- [ ] Inngest Cloud (managed) — free tier
- [ ] Clerk production instance (free tier)
- [ ] Cloudflare R2 bucket + presigned URLs
- [ ] Configure all environment variables
- [ ] End-to-end test with real drone imagery
- [ ] Record demo video

## Repository Structure

```
nadir-demo/
├── app/                              # Astro application
│   ├── src/
│   │   ├── db/
│   │   │   ├── index.ts              # Drizzle connection (pg Pool)
│   │   │   └── schema.ts             # Drizzle schema (users, projects, jobs)
│   │   ├── components/
│   │   │   ├── ui/                   # WebcoreUI wrappers
│   │   │   └── app/                  # App-specific components
│   │   │       ├── JobStatus.tsx
│   │   │       ├── ProjectCard.astro
│   │   │       └── ImageUpload.tsx
│   │   ├── layouts/
│   │   │   └── Layout.astro
│   │   ├── pages/
│   │   │   ├── index.astro
│   │   │   ├── dashboard.astro
│   │   │   ├── sign-in.astro
│   │   │   ├── sign-up.astro
│   │   │   ├── projects/
│   │   │   │   ├── new.astro
│   │   │   │   ├── [id].astro
│   │   │   │   └── [id]/results.astro
│   │   │   └── api/
│   │   │       ├── projects.ts
│   │   │       ├── projects/[id]/upload-url.ts
│   │   │       ├── projects/[id]/process.ts
│   │   │       ├── jobs/[id].ts
│   │   │       └── inngest.ts
│   │   ├── lib/
│   │   │   ├── users.ts              # User domain (Drizzle queries)
│   │   │   ├── projects.ts           # Project domain (Drizzle queries)
│   │   │   ├── jobs.ts               # Job domain (Drizzle queries)
│   │   │   ├── inngest.ts            # Inngest client
│   │   │   └── r2.ts                 # R2 storage helper
│   │   └── middleware.ts             # Clerk middleware
│   ├── drizzle/                      # Generated migration files
│   ├── astro.config.mjs
│   ├── drizzle.config.ts             # Drizzle Kit configuration
│   ├── package.json
│   └── .env.example
│
├── worker/                           # Docker worker
│   ├── Dockerfile                    # Pixi + Nadir + Node.js
│   ├── src/
│   │   ├── db/
│   │   │   ├── index.ts              # Drizzle connection (mirrors app)
│   │   │   └── schema.ts             # Minimal schema (jobs table)
│   │   ├── inngest.ts                # Inngest client
│   │   ├── functions/
│   │   │   └── process-project.ts    # Main function
│   │   ├── nadir.ts                  # Rust CLI subprocess wrapper
│   │   └── r2.ts                     # R2 download/upload
│   ├── package.json
│   └── pixi.toml                     # COLMAP, OpenMVS, GDAL, PROJ
│
├── docker-compose.yml                # Postgres locally
├── package.json                      # pnpm workspace root
└── README.md
```

## Local Development

```bash
# Terminal 1: Postgres
docker compose up -d postgres

# Terminal 2: Push schema to local DB
cd app
npx drizzle-kit push
# → creates tables from src/db/schema.ts

# Terminal 3: Astro app
pnpm install
pnpm dev
# → http://localhost:4321

# Terminal 4: Inngest dev server (auto-detects functions)
npx inngest-cli@latest dev
# → http://localhost:8288 (dashboard)

# Terminal 5: Worker (locally, no Docker needed for dev)
cd worker
pnpm install
pnpm dev
# → registers Inngest functions on localhost
```

**Migration workflow (when schema changes):**

```bash
# Generate migration files (tracked in git)
cd app
npx drizzle-kit generate

# Apply migrations to database
npx drizzle-kit migrate

# Or for quick local iteration (no migration files)
npx drizzle-kit push
```

## Environment Variables

```bash
# app/.env
PUBLIC_CLERK_PUBLISHABLE_KEY=pk_test_...
CLERK_SECRET_KEY=sk_test_...

# Server-side only — never use PUBLIC_DATABASE_URL
DATABASE_URL=postgresql://app:app@localhost:5432/app

INNGEST_EVENT_KEY=dev-key
INNGEST_SIGNING_KEY=dev-signing-key

R2_ACCOUNT_ID=...
R2_ACCESS_KEY_ID=...
R2_SECRET_ACCESS_KEY=...
R2_BUCKET=nadir-artifacts

PUBLIC_R2_PUBLIC_URL=https://cdn.example.com

# worker/.env
DATABASE_URL=postgresql://app:app@localhost:5432/app
INNGEST_EVENT_KEY=dev-key
INNGEST_SIGNING_KEY=dev-signing-key
R2_ACCOUNT_ID=...
R2_ACCESS_KEY_ID=...
R2_SECRET_ACCESS_KEY=...
R2_BUCKET=nadir-artifacts
```

**Never expose to client:** `CLERK_SECRET_KEY`, `DATABASE_URL`,
`INNGEST_SIGNING_KEY`, `R2_SECRET_ACCESS_KEY`.

## The Demo Script

```
1. Visit nadir.dev
2. Click "Sign in" → Clerk OAuth (Google/GitHub)
3. Click "New Project" → name it "Test Flight"
4. Drag & drop 50 drone images (direct upload to R2 via presigned URLs)
5. Click "Process" → 202 response, job queued in Inngest
6. Watch progress bar:
   "features 34% → sfm 67% → dense 89% → ortho 100%"
7. Click "View Results"
8. Interactive map shows orthomosaic overlay
9. Download GeoTIFF
10. "This is Astro + Clerk + Postgres + Drizzle + Inngest.
    The photogrammetry runs in a Docker worker.
    ~$30/month on Railway."
```

## Cost Estimate

| Service | Tier | Monthly Cost |
|---|---|---|
| Railway (Astro app, 1GB) | Pro | ~$5 |
| Railway (worker, 8GB) | Pro | ~$20 |
| Railway (Postgres) | Pro | ~$5 |
| Clerk | Free (10K MAU) | $0 |
| Inngest | Free (50K steps/mo) | $0 |
| Cloudflare R2 (10GB) | Free | $0 |
| Domain | Annual | ~$1 |
| **Total** | | **~$30/month** |

## Event Naming Convention

Follow `<domain>.<entity>.<action>`:

```
user.created                    # Clerk webhook → local user record
project.created
project.deleted
project.images.uploaded
project.process.requested       # → triggers worker
job.status.updated
job.completed
job.failed
```

## Dependency Direction Rules

```
components → pages → BFF → domain functions → Drizzle → PostgreSQL

BFF → Inngest event → worker → Drizzle → PostgreSQL

Never: database → UI
Never: worker → browser
Never: component → Drizzle or raw SQL
Never: raw SQL bypassing Drizzle (except jsonb_set helpers via sql``)
```

## Authorization Pattern

Every protected endpoint:

```
1. Authenticate  (Clerk userId from Astro.locals.auth())
2. Authorize     (verify user owns the resource via Drizzle query)
3. Validate      (Zod schema on input)
4. Execute       (call domain function in src/lib/)
5. Persist       (via Drizzle in src/db/)
6. Emit event    (if async work needed)
7. Return        (JSON response)
```

Never trust `userId` or `ownerId` from the browser when it can be derived
from the Clerk session.

## After the Demo

Once the demo proves the concept:

1. **Replace Nadir CLI's ODM wrapping with step-scoped engines** (COLMAP + OpenMVS + GDAL directly)
2. **Add content-addressed caching** in the Nadir CLI
3. **Add QC gates** (reprojection RMSE, coverage checks)
4. **Add adaptive planning** (matching strategy by image count)
5. **Add provenance UI** (`/projects/[id]/explain`)
6. **Split worker into domain workers** (recon, surface, cartography)
7. **Add organizations** (Clerk Organizations feature)
8. **Add PostGIS + pgvector extensions** via Drizzle custom migrations

The demo code is NOT thrown away. Astro app grows features. Worker grows
capabilities. Drizzle schema grows tables. Inngest functions grow steps.
Architecture stays the same.

## LLM Implementation Rules

When an LLM works on this demo:

1. Preserve the architecture (modular monolith, no microservices)
2. Prefer WebcoreUI components over custom CSS
3. Reuse Clerk for auth, never implement passwords
4. Use Drizzle ORM for all database access, never raw `pg` queries
5. Keep Drizzle schema in `src/db/schema.ts`, connection in `src/db/index.ts`
6. Use `drizzle-kit generate` + `migrate` for schema changes in production
7. Use `drizzle-kit push` for quick local iteration only
8. Use Inngest for async work, never a custom queue
9. Use Docker workers only for heavy/isolated workloads
10. Keep business logic in `src/lib/`, not in `.astro` files
11. Validate all external input with Zod
12. Authenticate AND authorize on every protected endpoint
13. Never expose `DATABASE_URL` or server secrets to the browser
14. Follow event naming: `<domain>.<entity>.<action>`
15. Store user-visible job state in Postgres via Drizzle
16. Keep workers stateless
17. Prefer boring code over clever abstractions

## See Also

- [Hosting comparison](./hosting-comparison.md) — Railway details
- [Docker strategy](./docker-strategy.md) — worker container model
- [Pixi setup](./pixi-setup.md) — Nadir CLI dependencies
- [V0 MVP roadmap](../roadmap/v0-mvp.md) — what comes after
