/**
 * TypeScript convenience client for the nadir engine.
 *
 * **Scaffold.** One function that delegates to `@nadir/sdk`, so the dependency edge between
 * the two packages is real and Turbo has something to order. That edge is the point: it is
 * what makes `client` build only after `sdk`, and it is the shape the API client will keep.
 */

import { describe as describeSdk } from "@nadir/sdk";

/** Describe the local nadir build, as a TypeScript client sees it. */
export function describe(): string {
  return `nadir/client: nadir/sdk: ${describeSdk()}`;
}
