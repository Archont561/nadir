---
type: Architecture
title: Worker Protocol
description: "Worker API, protobuf schema, transport independence, distributed execution"
purpose: Worker API, protobuf schema, transport independence, distributed execution
last_updated: 2025-02-23
status: stable
related:
  - overview.md
  - engine-registry.md
  - artifact-model.md
  - ../../backlog/docs/decisions/005-worker-topology-per-step.md
  - ../../backlog/docs/roadmaps/v2-scale.md
---

# Worker Protocol

## TL;DR

The Worker Protocol is the real product boundary of Nadir. It defines how
tasks are submitted, executed, and monitored. The protocol is transport-
independent: local IPC, HTTP, gRPC/Connect, or event broker are all
implementations of the same interface. The SaaS sits beside the protocol,
not inside it. No SaaS concepts (org, billing, project) leak into the
computational plane.

## The Core Insight

The standalone Worker API is the actual product boundary, not merely an
implementation detail of the SDK. Local and remote execution use the same
programming model. The only thing that changes is the transport.

```
const worker = createWorkerClient({ transport: "local" });
// vs.
const worker = createWorkerClient({ endpoint: "https://worker.example.com" });
// vs.
const worker = createWorkerClient({ endpoint: CLOUD_ENDPOINT, auth: token });

// Everything after this is identical:
const run = await worker.submit(nadir.mapping({ quality: "survey" }));
for await (const event of run.events()) { console.log(event); }
```

## The Worker Interface

```rust
pub trait Worker: Send + Sync {
    fn capabilities(&self) -> WorkerCapabilities;

    async fn submit(&self, task: Task) -> Result<Run>;

    async fn get_run(&self, id: RunId) -> Result<Run>;

    async fn cancel(&self, id: RunId) -> Result<()>;

    fn events(&self, id: RunId) -> impl Stream<Item = RunEvent>;
}
```

This is the stable API. Everything else is implementation.

## Key Types

### Task

```rust
pub struct Task {
    pub id: TaskId,
    pub kind: TaskKind,
    pub inputs: Vec<ArtifactRef>,
    pub config: Vec<u8>,           // serialized domain config
    pub resources: ResourceRequirements,
}

pub struct ResourceRequirements {
    pub cpu_cores: u32,
    pub memory_bytes: u64,
    pub gpu: bool,
    pub gpu_memory_bytes: Option<u64>,
    pub disk_bytes: u64,
}
```

### Run

```rust
pub struct Run {
    pub id: RunId,
    pub task_id: TaskId,
    pub status: RunStatus,
    pub progress: f32,
    pub stage: Option<String>,
    pub outputs: Vec<ArtifactRef>,
    pub error: Option<String>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
}

pub enum RunStatus {
    Queued,
    Running,
    Completed,
    Failed,
    Cancelled,
}
```

### Worker Capabilities

```rust
pub struct WorkerCapabilities {
    pub worker_id: String,
    pub version: String,
    pub engines: Vec<EngineCapability>,
    pub resources: ResourceProfile,
}

pub struct EngineCapability {
    pub name: String,
    pub version: String,
    pub tasks: Vec<TaskKind>,
}

pub struct ResourceProfile {
    pub cpu_cores: u32,
    pub memory_bytes: u64,
    pub gpu: Option<GpuInfo>,
    pub disk_bytes: u64,
}
```

### Events

```rust
pub enum RunEvent {
    Queued { run_id: RunId },
    Started { run_id: RunId },
    Progress { run_id: RunId, progress: f32, stage: String, message: String },
    ArtifactCreated { run_id: RunId, artifact: ArtifactRef },
    ArtifactPersisted { run_id: RunId, artifact: ArtifactRef },
    Completed { run_id: RunId, outputs: Vec<ArtifactRef> },
    Failed { run_id: RunId, error: String },
    Retrying { run_id: RunId, attempt: u32, reason: String },
}
```

## Protobuf Schema (V2.0)

The wire format is protobuf. One source of truth generates bindings for
Rust (`prost`), TypeScript (`@connectrpc`), and Python (`protobuf`).

```protobuf
syntax = "proto3";
package nadir.worker.v1;

service WorkerService {
  rpc GetCapabilities(GetCapabilitiesRequest)
      returns (GetCapabilitiesResponse);
  rpc SubmitTask(SubmitTaskRequest)
      returns (SubmitTaskResponse);
  rpc GetRun(GetRunRequest)
      returns (GetRunResponse);
  rpc CancelRun(CancelRunRequest)
      returns (CancelRunResponse);
  rpc SubscribeEvents(SubscribeEventsRequest)
      returns (stream RunEvent);
}

message ArtifactRef {
  string id = 1;
  string kind = 2;
  string checksum = 3;
  uint64 size = 4;
}

message Task {
  string id = 1;
  string kind = 2;
  repeated ArtifactRef inputs = 3;
  bytes config = 4;
  ResourceRequirements resources = 5;
}

message ResourceRequirements {
  uint32 cpu_cores = 1;
  uint64 memory_bytes = 2;
  bool gpu = 3;
  uint64 disk_bytes = 4;
}

message RunEvent {
  string run_id = 1;
  oneof event {
    ProgressEvent progress = 2;
    ArtifactEvent artifact = 3;
    CompletionEvent completion = 4;
    FailureEvent failure = 5;
  }
}
```

## Transport Independence

The protocol is transport-agnostic. Implementations:

```
Worker API
   │
   ├── Local IPC      (embedded in-process, V0.1)
   ├── HTTP/REST      (simple JSON, V0.1 demo)
   ├── gRPC/Connect   (protobuf, V2.0)
   ├── WebSocket      (event streaming, V2.0)
   └── Event Broker   (NATS JetStream, V3.0)
```

### Local transport (V0.1)

```rust
pub struct LocalWorker {
    executor: Executor,
    registry: EngineRegistry,
}

impl Worker for LocalWorker {
    async fn submit(&self, task: Task) -> Result<Run> {
        let run_id = RunId::new();
        let executor = self.executor.clone();
        tokio::spawn(async move {
            executor.execute_task(&task).await
        });
        Ok(Run::queued(run_id))
    }
}
```

### HTTP transport (V0.1 demo)

```rust
// Server side (Axum)
let app = Router::new()
    .route("/tasks", post(submit_task))
    .route("/tasks/:id", get(get_run))
    .route("/tasks/:id/cancel", post(cancel_run))
    .route("/tasks/:id/events", get(stream_events));

// Client side
pub struct HttpWorker {
    endpoint: String,
    client: reqwest::Client,
}

impl Worker for HttpWorker {
    async fn submit(&self, task: Task) -> Result<Run> {
        let resp = self.client
            .post(format!("{}/tasks", self.endpoint))
            .json(&task)
            .send().await?;
        Ok(resp.json().await?)
    }
}
```

## The WorkflowRuntime Abstraction

For durable workflow orchestration (V2.0+), define a runtime interface:

```rust
pub trait WorkflowRuntime: Send + Sync {
    async fn submit(&self, workflow: Workflow) -> Result<Run>;
    async fn cancel(&self, run: RunId) -> Result<()>;
    async fn get(&self, run: RunId) -> Result<Run>;
    fn events(&self, run: RunId) -> impl Stream<Item = RunEvent>;
}
```

Implementations:

```
LocalRuntime       → in-process executor (V0.1, standalone)
TemporalRuntime    → Temporal durable execution (V2.0, distributed)
CloudRuntime       → SaaS managed workers (V3.0, commercial)
```

This lets you use Temporal for retries, recovery, and workflow history
without exposing Temporal concepts in the public API.

## The Computational Plane vs. SaaS Control Plane

**Never allow SaaS concepts to enter the Worker Protocol.**

```
Computational plane (Worker Protocol)
────────────────────────────────────────
Task
Workflow
Run
Worker
Artifact
Engine
Capability

SaaS control plane (separate)
────────────────────────────────────────
Organization
User
Project
Subscription
Quota
Billing
Permissions
```

The SaaS **maps** its concepts onto the computational plane:

```rust
// SaaS layer wraps the worker protocol
pub struct CloudWorker {
    inner: HttpWorker,
    org_id: String,
    project_id: String,
    auth_token: String,
}

impl Worker for CloudWorker {
    async fn submit(&self, mut task: Task) -> Result<Run> {
        // SaaS adds metadata as headers, NOT in the task itself
        let resp = self.inner.client
            .post(format!("{}/tasks", self.endpoint))
            .header("X-Org-Id", &self.org_id)
            .header("X-Project-Id", &self.project_id)
            .bearer_auth(&self.auth_token)
            .json(&task)
            .send().await?;
        Ok(resp.json().await?)
    }
}
```

The worker doesn't know about orgs or billing. It just executes tasks.

## Execution Modes

The same API works across all deployment topologies:

```
Embedded/local
  App → SDK → LocalWorker → ODM

Local daemon
  App → WorkerClient → localhost:8080 → ODM

Remote worker
  App → WorkerClient → https://worker.example.com → ODM

Distributed cluster
  App → Scheduler → [Worker1, Worker2, Worker3] → ODM

Managed SaaS
  App → SaaS API → Managed Scheduler → Managed Workers → ODM
```

## Data Transfer Between Workers

Workers communicate through task protocol, artifact references, and events.
Not giant filesystem transfers.

```
Preferred:
  Worker A → Object Storage → Worker B

Avoid:
  Worker A → Worker B (direct transfer)

Optimization:
  If Worker A and Worker B share a volume, use local path.
  ArtifactLocation tracks all known locations.
```

## See Also

- [ADR-005: Per-step worker topology](../../backlog/docs/decisions/005-worker-topology-per-step.md)
- [Engine registry](./engine-registry.md) — what workers can execute
- [Artifact model](./artifact-model.md) — what flows between workers
- [V2 roadmap](../../backlog/docs/roadmaps/v2-scale.md) — when protobuf and distribution arrive
