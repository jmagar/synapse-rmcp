import type { CapabilitiesResult } from "./api";
import type { ActionScope } from "./template";

export type CapabilityState =
  | "anonymous"
  | "checking"
  | "read"
  | "write"
  | "expired"
  | "unavailable";

export interface RequestLease {
  signal: AbortSignal;
}

/** Synchronous request mutex plus abort ownership for React handlers. */
export class RequestCoordinator {
  private active: RequestLease | null = null;
  private controller: AbortController | null = null;

  begin(): RequestLease | null {
    if (this.active) return null;
    this.controller = new AbortController();
    this.active = { signal: this.controller.signal };
    return this.active;
  }

  isCurrent(lease: RequestLease): boolean {
    return this.active === lease && !lease.signal.aborted;
  }

  finish(lease: RequestLease): void {
    if (!this.isCurrent(lease)) return;
    this.active = null;
    this.controller = null;
  }

  cancel(): void {
    this.controller?.abort();
    this.controller = null;
    this.active = null;
  }
}

export function capabilityStateFor(
  result: CapabilitiesResult | undefined,
  status: number | undefined,
  hasToken: boolean,
  error?: string,
): CapabilityState {
  if (result?.scopes.includes("synapse:write")) return "write";
  if (result?.scopes.includes("synapse:read")) return "read";
  if (hasToken && (status === 401 || status === 403)) return "expired";
  if (!hasToken && (status === 401 || status === 403)) return "anonymous";
  if (error || (status !== undefined && status >= 400)) return "unavailable";
  return "anonymous";
}

export function scopeAllowed(state: CapabilityState, scope: ActionScope): boolean {
  if (scope === "public") return true;
  if (scope === "synapse:read") return state === "read" || state === "write";
  return state === "write";
}
