// LAN transport client for the protocol v0.1 resources (docs/protocol.md).
//
// The companion only ever talks to the device origin it was served from.
// No cloud fallback, no analytics, no third-party endpoints. Every method
// returns the parsed envelope; the state layer applies the version gate.
// The SSE source is injectable so tests (and USB-only environments) can
// run without EventSource support.

import { buildRequest, type Observation } from "./protocol.ts";

export interface StatusPayload {
  protocol_version: string;
  device_id: string;
  uptime_ms: number;
  calibration_state: string;
  faults: string[];
  wifi_connected: boolean;
  storage: { used: number; total: number };
  [extra: string]: unknown; // additive minor-version fields are tolerated
}

export interface Envelope<T> {
  protocol_version: string;
  request_id: string;
  ok: boolean;
  data?: T;
  error?: { code: string; message: string };
}

export class DeviceRequestError extends Error {
  constructor(
    public code: string,
    message: string,
  ) {
    super(message);
    this.name = "DeviceRequestError";
  }
}

export interface ClientOptions {
  /** Defaults to same-origin (the device itself). */
  origin?: string;
  fetchImpl?: typeof fetch;
  /** Injectable EventSource constructor for SSE tests. */
  eventSourceFactory?: (url: string) => EventSourceLike;
}

export interface EventSourceLike {
  onmessage: ((event: { data: string }) => void) | null;
  onerror: ((event: unknown) => void) | null;
  close(): void;
}

/** Real EventSource is structurally close enough for our narrow use. */
function asSourceLike(candidate: unknown): EventSourceLike {
  return candidate as EventSourceLike;
}

export class DeviceClient {
  private readonly origin: string;
  private readonly doFetch: typeof fetch;
  private readonly sseFactory?: (url: string) => EventSourceLike;

  constructor(options: ClientOptions = {}) {
    this.origin = (options.origin ?? "").replace(/\/$/, "");
    const impl = options.fetchImpl ?? globalThis.fetch;
    if (typeof impl !== "function") {
      throw new Error("fetch is unavailable in this environment");
    }
    this.doFetch = impl.bind(globalThis);
    this.sseFactory = options.eventSourceFactory;
  }

  /** Last request builder id, echoed for bounded correlation. */
  async request<T>(path: string, init?: RequestInit): Promise<T> {
    const response = await this.doFetch(`${this.origin}${path}`, {
      ...init,
      headers: { accept: "application/json", ...(init?.headers ?? {}) },
    });
    if (!response.ok && response.status >= 500) {
      throw new DeviceRequestError(
        "internal_error",
        `Device returned ${response.status}`,
      );
    }
    const envelope = (await response.json()) as Envelope<T>;
    if (!envelope.ok) {
      throw new DeviceRequestError(
        envelope.error?.code ?? "internal_error",
        envelope.error?.message ?? "Device rejected the request",
      );
    }
    if (envelope.data === undefined) {
      throw new DeviceRequestError(
        "malformed_request",
        "Envelope missing data",
      );
    }
    return envelope.data;
  }

  status(): Promise<StatusPayload> {
    return this.request<StatusPayload>("/api/v1/status");
  }

  snapshot(): Promise<Observation> {
    return this.request<Observation>("/api/v1/snapshot");
  }

  observations(limit = 100): Promise<Observation[]> {
    return this.request<Observation[]>(
      `/api/v1/observations?limit=${encodeURIComponent(String(limit))}`,
    );
  }

  addEvent(kind: string, detail: string): Promise<unknown> {
    const body = buildRequest("event", { kind, detail });
    return this.request("/api/v1/events", {
      method: "POST",
      headers: { "content-type": "application/json" },
      body,
    });
  }

  /**
   * Calibration steps per frozen LAN table. Body uses the shared RPC
   * grammar so the firmware LAN adapter (issue #6) maps 1:1 to the same
   * handlers as USB CDC: calibration_begin_tare | calibration_begin_reference.
   */
  calibrationStep(
    step: "tare" | "reference",
    params: Record<string, unknown> = {},
  ): Promise<{ token?: number; state?: string }> {
    const op =
      step === "tare"
        ? "calibration_begin_tare"
        : "calibration_begin_reference";
    return this.request(`/api/v1/calibration/${step}`, {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: buildRequest(op, params),
    });
  }

  exportUrl(format: "csv" | "json"): string {
    return `${this.origin}/api/v1/export?format=${format}`;
  }

  backup(): Promise<unknown> {
    return this.request("/api/v1/backup");
  }

  restoreValidate(doc: unknown): Promise<{ digest: number; preview: unknown }> {
    return this.request("/api/v1/restore/validate", {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify(doc),
    });
  }

  restoreCommit(digest: number): Promise<unknown> {
    return this.request("/api/v1/restore/commit", {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({ digest }),
    });
  }

  /**
   * Scoped deletion. The confirmation `token` is granted by the device
   * during an active presence session (protocol.md mutation rules); the
   * UI must never fabricate one, which is why deletion guides the user
   * through on-device confirmation until issue #6 exposes a token route.
   */
  deleteScope(scope: string, token: number): Promise<unknown> {
    return this.request(`/api/v1/data?scope=${encodeURIComponent(scope)}`, {
      method: "DELETE",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({ token }),
    });
  }

  /**
   * Open the SSE stream if supported; returns null in environments without
   * an EventSource implementation (the UI then polls the snapshot).
   */
  openStream(
    onObservation: (obs: Observation) => void,
    onError: () => void,
  ): (() => void) | null {
    const url = `${this.origin}/api/v1/stream`;
    let source: EventSourceLike;
    if (this.sseFactory) {
      source = this.sseFactory(url);
    } else if (typeof globalThis.EventSource === "function") {
      source = asSourceLike(new globalThis.EventSource(url));
    } else {
      return null;
    }
    source.onmessage = (event) => {
      try {
        onObservation(JSON.parse(event.data) as Observation);
      } catch {
        // Malformed frame: ignore; the stream keeps running.
      }
    };
    source.onerror = () => onError();
    return () => source.close();
  }
}
