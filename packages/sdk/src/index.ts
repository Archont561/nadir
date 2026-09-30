/**
 * TypeScript client for the nadir pipeline API.
 *
 * **Scaffold.** One type and one function, so the package typechecks under the workspace
 * `tsconfig` rules and has a test that fails if the wiring is broken. The real surface is
 * the pipeline API: submit a dataset, poll a run, fetch artifacts.
 *
 * The reason this package exists separately from `@nadir/client` is a boundary that will
 * matter later: `sdk` is the wire contract, `client` is the browser-facing convenience
 * layer over it. Keeping them apart means a change to how a run is polled does not force a
 * change to what the wire format is.
 */

/** A pipeline run, as the API reports it. */
export interface Run {
  /** Stable identifier, safe to use as a cache key. */
  readonly id: string;
  /** Where the run has got to. `succeeded` and `failed` are terminal. */
  readonly status: "pending" | "running" | "succeeded" | "failed";
  /** Human-readable reason, present only when `status` is `failed`. */
  readonly error?: string;
}

/** Describe the local nadir build. */
export function describe(): string {
  return "nadir/sdk: pipeline API client";
}
