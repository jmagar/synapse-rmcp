/**
 * Typed client for the Synapse2 REST API.
 *
 * All actions are dispatched via POST /v1/synapse2 with:
 *   { "action": "<action>", "params": { ... } }
 *
 * The base URL is relative (empty string) so the same binary serves
 * both the API and the web UI without CORS configuration.
 */

import { endpoint, WEB_APP_CONFIG } from "@/lib/template";

export interface ApiResponse<T = unknown> {
  data?: T;
  error?: string;
  status?: number;
}

// Deliberately memory-only. Persisting bearer/OAuth credentials in Web Storage
// exposes them to every same-origin script for the lifetime of the tab.
let bearerToken: string | null = null;

export function getBearerToken(): string | null {
  return bearerToken;
}

export function setBearerToken(token: string): void {
  const normalized = token.trim();
  bearerToken = normalized || null;
}

export function clearBearerToken(): void {
  bearerToken = null;
}

export interface StatusResult {
  status: string;
  note?: string;
  server?: string;
  version?: string;
  transport?: string;
}

export interface HealthResult {
  status: string;
}

export interface ActivityEvent {
  sequence: number;
  timestamp: string;
  transport: string;
  action: string;
  ok: boolean;
  error?: string;
}

export interface ActivityResult {
  events: ActivityEvent[];
}

export interface CapabilitiesResult {
  scopes: string[];
  destructive_allowed: boolean;
}

/** Shared fetch helper — handles JSON parsing and error normalisation. */
export async function apiFetch<T>(
  url: string,
  options?: RequestInit,
  validate?: (value: unknown) => value is T,
): Promise<ApiResponse<T>> {
  try {
    const res = await fetch(url, options);
    const text = await res.text();
    const json = parseJsonBody(text);
    if (!res.ok) {
      const error =
        isRecord(json) && typeof json.error === "string" ? json.error : `HTTP ${res.status}`;
      return { error, status: res.status };
    }
    if (validate && !validate(json)) {
      return { error: "Invalid response payload", status: res.status };
    }
    return { data: json as T };
  } catch (e) {
    return { error: e instanceof Error ? e.message : "Network error" };
  }
}

export function parseJsonBody(text: string): unknown {
  if (!text.trim()) return {};
  try {
    return JSON.parse(text);
  } catch {
    return text;
  }
}

export function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function isActivityEvent(value: unknown): value is ActivityEvent {
  return (
    isRecord(value) &&
    typeof value.sequence === "number" &&
    Number.isSafeInteger(value.sequence) &&
    value.sequence >= 0 &&
    typeof value.timestamp === "string" &&
    typeof value.transport === "string" &&
    typeof value.action === "string" &&
    typeof value.ok === "boolean" &&
    (value.error === undefined || typeof value.error === "string")
  );
}

function isActivityResult(value: unknown): value is ActivityResult {
  return isRecord(value) && Array.isArray(value.events) && value.events.every(isActivityEvent);
}

function isCapabilitiesResult(value: unknown): value is CapabilitiesResult {
  return (
    isRecord(value) &&
    Array.isArray(value.scopes) &&
    value.scopes.every((scope) => typeof scope === "string") &&
    typeof value.destructive_allowed === "boolean"
  );
}

/** POST /v1/synapse2 — dispatch an action */
export function callAction<T = unknown>(
  action: string,
  params: Record<string, unknown> = {},
  signal?: AbortSignal,
): Promise<ApiResponse<T>> {
  const token = getBearerToken();
  return apiFetch<T>(endpoint(WEB_APP_CONFIG.restEndpoint), {
    method: "POST",
    credentials: "same-origin",
    headers: {
      "Content-Type": "application/json",
      ...(token ? { Authorization: `Bearer ${token}` } : {}),
    },
    body: JSON.stringify({ action, params }),
    signal,
  });
}

/** GET /capabilities — authoritative scopes for the current browser credential. */
export function getCapabilities(signal?: AbortSignal): Promise<ApiResponse<CapabilitiesResult>> {
  const token = getBearerToken();
  return apiFetch<CapabilitiesResult>(
    endpoint(WEB_APP_CONFIG.capabilitiesEndpoint),
    {
      signal,
      credentials: "same-origin",
      headers: token ? { Authorization: `Bearer ${token}` } : {},
    },
    isCapabilitiesResult,
  );
}

/** GET /health */
export function getHealth(signal?: AbortSignal): Promise<ApiResponse<HealthResult>> {
  return apiFetch<HealthResult>(endpoint(WEB_APP_CONFIG.healthEndpoint), { signal });
}

/** GET /status */
export function getStatus(signal?: AbortSignal): Promise<ApiResponse<StatusResult>> {
  return apiFetch<StatusResult>(endpoint(WEB_APP_CONFIG.statusEndpoint), { signal });
}

/** GET /activity — shared bounded REST/MCP audit stream. */
export function getActivity(signal?: AbortSignal): Promise<ApiResponse<ActivityResult>> {
  const token = getBearerToken();
  return apiFetch<ActivityResult>(
    endpoint(WEB_APP_CONFIG.activityEndpoint),
    {
      signal,
      credentials: "same-origin",
      headers: token ? { Authorization: `Bearer ${token}` } : {},
    },
    isActivityResult,
  );
}
