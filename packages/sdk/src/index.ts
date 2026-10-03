/** Native TypeScript face of the nadir engine. */
import addon from "./native.js";

export const TRANSPORT_VERSION = 1;
export type Operation = "ping" | "version" | "describe";
export type Payload = Record<string, unknown>;

export interface EngineResponse<TResult extends Payload = Payload> {
  transportVersion: number;
  ok: boolean;
  result: TResult;
}

export class EngineError extends Error {
  readonly detail: Payload;

  constructor(detail: Payload) {
    super(typeof detail.error === "string" ? detail.error : JSON.stringify(detail));
    this.name = "EngineError";
    this.detail = detail;
  }
}

export function invokeRaw<TResult extends Payload = Payload>(
  operation: Operation,
  payload: Payload = {},
): EngineResponse<TResult> {
  return JSON.parse(
    addon.invoke(JSON.stringify({ transportVersion: TRANSPORT_VERSION, operation, payload })),
  ) as EngineResponse<TResult>;
}

export function invoke<TResult extends Payload = Payload>(
  operation: Operation,
  payload: Payload = {},
): TResult {
  const response = invokeRaw<TResult>(operation, payload);
  if (!response.ok) throw new EngineError(response.result);
  return response.result;
}

export function ping(message = "typescript"): string {
  return invoke<{ echo: { message: string } }>("ping", { message }).echo.message;
}

export function version(): { version: string; transportVersion: number; coreCrate: string } {
  return invoke("version");
}

export function describe(): string {
  return invoke<{ description: string; stage: string }>("describe").description;
}
