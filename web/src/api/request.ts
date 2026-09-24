type AnyResponse = { status: number; data: unknown; };

export type Outcome = { ok: true; } | { ok: false; message: string; };

export class ApiRequestError extends Error {
  readonly status: number;

  constructor(status: number, message: string) {
    super(message);
    this.name = "ApiRequestError";
    this.status = status;
  }
}

const errorMessage = (data: unknown, status: number) =>
  (typeof data === "object" && data !== null && "message" in data && typeof data.message === "string"
    ? data.message
    : `request failed with status ${status}`);

/** A read that fails throws, so the nearest `Errored` boundary shows it. */
export const failure = (response: AnyResponse) =>
  new ApiRequestError(response.status, errorMessage(response.data, response.status));

/** A write that fails is expected: a wrong admin key or a stale reorder is the reader's to fix. */
export const outcome = (response: AnyResponse): Outcome =>
  (response.status === 200 ? { ok: true } : { ok: false, message: errorMessage(response.data, response.status) });
