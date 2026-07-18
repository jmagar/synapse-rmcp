"use client";

import { useCallback, useEffect, useRef, useState } from "react";
import { ActionButton } from "@/components/dashboard/action-button";
import { Card } from "@/components/dashboard/card";
import { Button } from "@/components/ui/button";
import {
  callAction,
  getActivity,
  getBearerToken,
  getCapabilities,
  getHealth,
  getStatus,
  type StatusResult,
} from "@/lib/api";
import { type CapabilityState, capabilityStateFor, scopeAllowed } from "@/lib/request-state";
import { type ActionScope, WEB_APP_CONFIG } from "@/lib/template";

type HealthState = "ok" | "error" | "loading";

interface ActivityItem {
  id: string;
  time: string;
  action: string;
  result: string;
  ok: boolean;
}

const QUICK_ACTIONS: ReadonlyArray<{ id: string; label: string; scope: ActionScope }> = [
  { id: "help", label: "Help", scope: "public" },
  { id: "flux.docker.df", label: "Docker Disk", scope: "synapse:read" },
  { id: "scout.nodes", label: "Scout Nodes", scope: "synapse:read" },
];

export default function DashboardPage() {
  const [health, setHealth] = useState<HealthState>("loading");
  const [serverStatus, setServerStatus] = useState<StatusResult | null>(null);
  const [activity, setActivity] = useState<ActivityItem[]>([]);
  const [activityError, setActivityError] = useState<string | null>(null);
  const [quickActionError, setQuickActionError] = useState<string | null>(null);
  const [capability, setCapability] = useState<CapabilityState>("checking");
  const [runningActions, setRunningActions] = useState<Set<string>>(() => new Set());
  const runningActionsRef = useRef(new Set<string>());
  const actionControllersRef = useRef(new Map<string, AbortController>());
  const activityControllerRef = useRef<AbortController | null>(null);
  const activityGenerationRef = useRef(0);
  const mountedRef = useRef(true);

  const checkHealth = useCallback(async (signal?: AbortSignal) => {
    const res = await getHealth(signal);
    if (signal?.aborted) return;
    setHealth(res.data?.status === "ok" ? "ok" : "error");
  }, []);

  const checkStatus = useCallback(async (signal?: AbortSignal) => {
    const res = await getStatus(signal);
    if (signal?.aborted) return;
    if (res.data) setServerStatus(res.data);
  }, []);

  useEffect(() => {
    const controller = new AbortController();
    let timer: ReturnType<typeof setTimeout> | undefined;
    const poll = async () => {
      await checkHealth(controller.signal);
      if (!controller.signal.aborted) timer = setTimeout(poll, 10_000);
    };
    void poll();
    void checkStatus(controller.signal);
    return () => {
      controller.abort();
      if (timer) clearTimeout(timer);
    };
  }, [checkHealth, checkStatus]);

  useEffect(() => {
    const controller = new AbortController();
    const checkCapabilities = async () => {
      const result = await getCapabilities(controller.signal);
      if (controller.signal.aborted) return;
      setCapability(
        capabilityStateFor(result.data, result.status, Boolean(getBearerToken()), result.error),
      );
    };
    void checkCapabilities();
    return () => controller.abort();
  }, []);

  const refreshActivity = useCallback(async () => {
    activityControllerRef.current?.abort();
    const controller = new AbortController();
    activityControllerRef.current = controller;
    const generation = ++activityGenerationRef.current;
    const result = await getActivity(controller.signal);
    if (controller.signal.aborted || generation !== activityGenerationRef.current) {
      return;
    }
    if (!result.data) {
      setActivityError(describeApiError(result.error, result.status));
      return;
    }
    setActivityError(null);
    setActivity(
      result.data.events.map((event) => ({
        id: `server:${event.sequence}`,
        time: new Date(event.timestamp).toLocaleTimeString(),
        action: `${event.transport}:${event.action}`,
        result: event.error ?? (event.ok ? "completed" : "failed"),
        ok: event.ok,
      })),
    );
  }, []);

  useEffect(() => {
    mountedRef.current = true;
    let timer: ReturnType<typeof setTimeout> | undefined;
    const poll = async () => {
      await refreshActivity();
      if (mountedRef.current) timer = setTimeout(poll, 5_000);
    };
    void poll();
    return () => {
      mountedRef.current = false;
      activityControllerRef.current?.abort();
      for (const controller of actionControllersRef.current.values()) controller.abort();
      if (timer) clearTimeout(timer);
    };
  }, [refreshActivity]);

  const runQuickAction = useCallback(
    async (quickAction: (typeof QUICK_ACTIONS)[number]) => {
      const { id, scope } = quickAction;
      if (!scopeAllowed(capability, scope)) {
        setQuickActionError(
          capability === "unavailable"
            ? "Server capabilities are unavailable. Try again when the server is reachable."
            : "This quick action requires a browser credential with read access. Open Tool Runner to authenticate.",
        );
        return;
      }
      if (runningActionsRef.current.has(id)) return;
      runningActionsRef.current.add(id);
      setRunningActions(new Set(runningActionsRef.current));
      setQuickActionError(null);
      const controller = new AbortController();
      actionControllersRef.current.set(id, controller);
      try {
        const result = await callAction(id, {}, controller.signal);
        if (controller.signal.aborted) return;
        if (result.error) {
          setQuickActionError(describeApiError(result.error, result.status));
          return;
        }
        await refreshActivity();
      } finally {
        actionControllersRef.current.delete(id);
        runningActionsRef.current.delete(id);
        if (mountedRef.current) setRunningActions(new Set(runningActionsRef.current));
      }
    },
    [capability, refreshActivity],
  );

  const readActionsDisabled = !scopeAllowed(capability, "synapse:read");

  const statusColor: Record<HealthState, string> = {
    ok: "var(--aurora-success)",
    error: "var(--aurora-error)",
    loading: "var(--aurora-text-muted)",
  };

  const statusLabel: Record<HealthState, string> = {
    ok: "Healthy",
    error: "Unreachable",
    loading: "Checking…",
  };

  return (
    <div className="max-w-5xl mx-auto space-y-6">
      {/* Header */}
      <div>
        <h1
          style={{
            fontFamily: "var(--aurora-font-display)",
            color: "var(--aurora-text-primary)",
            fontSize: "1.75rem",
            fontWeight: 700,
            marginBottom: "0.25rem",
          }}
        >
          {WEB_APP_CONFIG.dashboardTitle}
        </h1>
        <p style={{ color: "var(--aurora-text-muted)", fontSize: "0.875rem" }}>
          {WEB_APP_CONFIG.displayName} MCP server — real-time status and safe quick actions
        </p>
      </div>

      {/* Status cards */}
      <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
        <Card title="Server Health">
          <div className="flex items-center gap-2">
            <div
              style={{
                width: 10,
                height: 10,
                borderRadius: "50%",
                background: statusColor[health],
                boxShadow: health === "ok" ? `0 0 6px var(--aurora-success)` : undefined,
              }}
            />
            <span style={{ color: statusColor[health], fontWeight: 600, fontSize: "1rem" }}>
              {statusLabel[health]}
            </span>
          </div>
          <p
            style={{ color: "var(--aurora-text-muted)", fontSize: "0.75rem", marginTop: "0.5rem" }}
          >
            Polls {WEB_APP_CONFIG.healthEndpoint} every 10s
          </p>
        </Card>

        <Card title="Runtime">
          <p
            style={{
              color: "var(--aurora-accent-primary)",
              fontFamily: "var(--aurora-font-mono)",
              fontSize: "0.8rem",
              wordBreak: "break-all",
            }}
          >
            {serverStatus?.server ?? WEB_APP_CONFIG.serviceName}
          </p>
          {serverStatus?.version && (
            <p style={{ color: "var(--aurora-text-muted)", fontSize: "0.75rem" }}>
              v{serverStatus.version}
            </p>
          )}
        </Card>

        <Card title="Status">
          <p
            style={{
              color:
                serverStatus?.status === "ok"
                  ? "var(--aurora-success)"
                  : "var(--aurora-text-muted)",
              fontWeight: 600,
            }}
          >
            {serverStatus?.status ?? "—"}
          </p>
          {serverStatus?.note && (
            <p
              style={{
                color: "var(--aurora-text-muted)",
                fontSize: "0.75rem",
                marginTop: "0.25rem",
              }}
            >
              {serverStatus.note}
            </p>
          )}
          {serverStatus?.transport && (
            <p
              style={{
                color: "var(--aurora-text-muted)",
                fontSize: "0.75rem",
                marginTop: "0.25rem",
              }}
            >
              {serverStatus.transport}
            </p>
          )}
        </Card>
      </div>

      {/* Quick actions */}
      <div
        style={{
          background: "var(--aurora-panel-medium)",
          border: "1px solid var(--aurora-border-default)",
          borderRadius: "var(--radius-lg)",
          padding: "1.25rem",
        }}
      >
        <h2
          style={{
            color: "var(--aurora-text-muted)",
            fontWeight: 600,
            marginBottom: "1rem",
            fontSize: "0.9rem",
            textTransform: "uppercase",
            letterSpacing: "0.05em",
          }}
        >
          Quick Actions
        </h2>
        <div className="flex flex-wrap gap-3">
          {QUICK_ACTIONS.map((quickAction) => (
            <ActionButton
              key={quickAction.id}
              onClick={() => void runQuickAction(quickAction)}
              label={quickAction.label}
              loading={runningActions.has(quickAction.id)}
              disabled={!scopeAllowed(capability, quickAction.scope)}
            />
          ))}
          <Button asChild variant="neutral">
            <a href="/tools/">Open Tool Runner →</a>
          </Button>
        </div>
        {quickActionError && (
          <p role="alert" className="mt-3 text-sm" style={{ color: "var(--aurora-error)" }}>
            {quickActionError}
          </p>
        )}
        {!quickActionError && readActionsDisabled && capability !== "checking" && (
          <p className="mt-3 text-sm" style={{ color: "var(--aurora-text-muted)" }}>
            {capability === "unavailable"
              ? "Server capabilities are unavailable; protected quick actions are disabled."
              : "Protected quick actions require read access. Open Tool Runner to authenticate."}
          </p>
        )}
      </div>

      {/* Activity feed */}
      <div
        style={{
          background: "var(--aurora-panel-medium)",
          border: "1px solid var(--aurora-border-default)",
          borderRadius: "var(--radius-lg)",
          padding: "1.25rem",
        }}
      >
        <h2
          style={{
            color: "var(--aurora-text-muted)",
            fontWeight: 600,
            marginBottom: "1rem",
            fontSize: "0.9rem",
            textTransform: "uppercase",
            letterSpacing: "0.05em",
          }}
        >
          Recent Activity
        </h2>
        {activityError ? (
          <p role="alert" style={{ color: "var(--aurora-error)", fontSize: "0.875rem" }}>
            {activityError}
          </p>
        ) : activity.length === 0 ? (
          <p style={{ color: "var(--aurora-text-muted)", fontSize: "0.875rem" }}>
            No shared REST or MCP activity yet.
          </p>
        ) : (
          <div className="space-y-2">
            {activity.map((item) => (
              <div
                key={item.id}
                style={{
                  display: "flex",
                  gap: "0.75rem",
                  alignItems: "flex-start",
                  padding: "0.5rem 0.75rem",
                  background: "var(--aurora-control-surface)",
                  borderRadius: "var(--radius-sm)",
                  border: "1px solid var(--aurora-border-default)",
                }}
              >
                <span
                  style={{
                    color: item.ok ? "var(--aurora-success)" : "var(--aurora-error)",
                    fontFamily: "var(--aurora-font-mono)",
                    fontSize: "0.75rem",
                    minWidth: "4rem",
                  }}
                >
                  {item.time}
                </span>
                <span
                  style={{
                    color: "var(--aurora-accent-primary)",
                    fontFamily: "var(--aurora-font-mono)",
                    fontSize: "0.75rem",
                    minWidth: "8rem",
                  }}
                >
                  {item.action}
                </span>
                <span style={{ color: "var(--aurora-text-primary)", fontSize: "0.8rem", flex: 1 }}>
                  {item.result}
                </span>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}

function describeApiError(error: string | undefined, status: number | undefined): string {
  if (status === 401) return "Authentication is required or the browser credential has expired.";
  if (status === 403) return "The browser credential lacks permission for this operation.";
  return status ? `${error ?? "Request failed"} (HTTP ${status})` : (error ?? "Request failed");
}
