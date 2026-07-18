"use client";

import { useCallback, useEffect, useRef, useState } from "react";
import { ParamInput } from "@/components/tools/param-input";
import { ResponsePanel } from "@/components/tools/response-panel";
import { SubmitButton } from "@/components/tools/submit-button";
import { Button } from "@/components/ui/button";
import {
  callAction,
  clearBearerToken,
  getBearerToken,
  getCapabilities,
  setBearerToken,
} from "@/lib/api";
import {
  type CapabilityState,
  capabilityStateFor,
  RequestCoordinator,
  scopeAllowed,
} from "@/lib/request-state";
import {
  type ActionParam,
  DEFAULT_REST_ACTION,
  REST_ACTIONS,
  type RestActionId,
  WEB_APP_CONFIG,
} from "@/lib/template";

export default function ToolsPage() {
  const [selectedAction, setSelectedAction] = useState<RestActionId>(DEFAULT_REST_ACTION.id);
  const [paramValues, setParamValues] = useState<Record<string, string>>({});
  const [response, setResponse] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [isError, setIsError] = useState(false);
  const [tokenDraft, setTokenDraft] = useState("");
  const [capability, setCapability] = useState<CapabilityState>("checking");
  const [destructiveAllowed, setDestructiveAllowed] = useState(false);
  const requestsRef = useRef(new RequestCoordinator());
  const capabilityControllerRef = useRef<AbortController | null>(null);

  const verifyCredential = useCallback(async (hasToken: boolean) => {
    capabilityControllerRef.current?.abort();
    const controller = new AbortController();
    capabilityControllerRef.current = controller;
    setCapability("checking");
    const result = await getCapabilities(controller.signal);
    if (controller.signal.aborted) return;
    setCapability(capabilityStateFor(result.data, result.status, hasToken, result.error));
    setDestructiveAllowed(Boolean(result.data?.destructive_allowed));
  }, []);

  useEffect(() => {
    void verifyCredential(Boolean(getBearerToken()));
    return () => {
      capabilityControllerRef.current?.abort();
      requestsRef.current.cancel();
    };
  }, [verifyCredential]);

  const action = REST_ACTIONS.find((a) => a.id === selectedAction) ?? DEFAULT_REST_ACTION;
  const requestPreview = {
    action: selectedAction,
    params: buildParams(action.params, paramValues),
  };

  const handleSelect = (id: RestActionId) => {
    requestsRef.current.cancel();
    setLoading(false);
    setSelectedAction(id);
    setParamValues({});
    setResponse(null);
    setIsError(false);
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    const requestAction = selectedAction;
    const lease = requestsRef.current.begin();
    if (!lease) return;
    setLoading(true);
    setResponse(null);
    setIsError(false);

    const params = buildParams(action.params, paramValues);

    const res = await callAction(requestAction, params, lease.signal);
    if (!requestsRef.current.isCurrent(lease)) return;

    if (res.error) {
      const authHint =
        res.status === 401
          ? "The browser credential is missing, expired, or invalid."
          : res.status === 403
            ? "The credential is valid but lacks the scope required for this action."
            : undefined;
      setResponse(
        JSON.stringify({ error: res.error, status: res.status, hint: authHint }, null, 2),
      );
      setIsError(true);
    } else {
      setResponse(JSON.stringify(res.data, null, 2));
    }
    requestsRef.current.finish(lease);
    setLoading(false);
  };

  const saveCredential = () => {
    setBearerToken(tokenDraft);
    void verifyCredential(Boolean(tokenDraft.trim()));
    setTokenDraft("");
  };

  const removeCredential = () => {
    capabilityControllerRef.current?.abort();
    clearBearerToken();
    setCapability("anonymous");
    setDestructiveAllowed(false);
    setTokenDraft("");
  };

  return (
    <div className="max-w-4xl mx-auto space-y-6">
      {/* Header */}
      <div>
        <h1
          style={{
            fontFamily: "var(--aurora-font-display)",
            fontSize: "1.75rem",
            fontWeight: 700,
            marginBottom: "0.25rem",
          }}
        >
          Tool Runner
        </h1>
        <p style={{ color: "var(--aurora-text-muted)", fontSize: "0.875rem" }}>
          Call any action via{" "}
          <code
            style={{
              fontFamily: "var(--aurora-font-mono)",
              background: "var(--aurora-panel-strong)",
              padding: "0.1em 0.4em",
              borderRadius: "4px",
              fontSize: "0.8em",
            }}
          >
            POST {WEB_APP_CONFIG.restEndpoint}
          </code>
        </p>
      </div>

      <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
        {/* Action selector */}
        <div
          style={{
            background: "var(--aurora-panel-medium)",
            border: "1px solid var(--aurora-border-default)",
            borderRadius: "var(--radius-lg)",
            padding: "1rem",
          }}
        >
          <p
            style={{
              color: "var(--aurora-text-muted)",
              fontSize: "0.75rem",
              fontWeight: 600,
              textTransform: "uppercase",
              letterSpacing: "0.05em",
              marginBottom: "0.75rem",
            }}
          >
            Actions
          </p>
          <div className="space-y-1">
            {REST_ACTIONS.map((a) => (
              <Button
                type="button"
                key={a.id}
                onClick={() => handleSelect(a.id)}
                disabled={loading}
                variant="ghost"
                size="sm"
                className="w-full justify-start border-l-2 font-[var(--aurora-font-mono)]"
                style={{
                  textAlign: "left",
                  background: selectedAction === a.id ? "var(--aurora-hover-bg)" : "transparent",
                  color:
                    selectedAction === a.id
                      ? "var(--aurora-accent-primary)"
                      : "var(--aurora-text-primary)",
                  borderLeft:
                    selectedAction === a.id
                      ? "2px solid var(--aurora-accent-primary)"
                      : "2px solid transparent",
                }}
              >
                {a.label}
              </Button>
            ))}
          </div>
        </div>

        {/* Form + response */}
        <div className="md:col-span-2 space-y-4">
          <section
            aria-label="Browser authentication"
            className="rounded-lg border p-4"
            style={{ background: "var(--aurora-panel-medium)" }}
          >
            <p className="text-sm font-semibold">Browser authentication</p>
            <p className="mt-1 text-xs" style={{ color: "var(--aurora-text-muted)" }}>
              {credentialMessage(capability)}
            </p>
            <div className="mt-3 flex gap-2">
              <input
                aria-label="Bearer token"
                type="password"
                value={tokenDraft}
                onChange={(event) => setTokenDraft(event.target.value)}
                placeholder="Bearer token"
                className="min-w-0 flex-1 rounded-md border px-3 py-2 text-sm"
                style={{ background: "var(--aurora-control-surface)" }}
              />
              <Button type="button" onClick={saveCredential} disabled={!tokenDraft.trim()}>
                Save
              </Button>
              {getBearerToken() && (
                <Button type="button" variant="ghost" onClick={removeCredential}>
                  Clear
                </Button>
              )}
            </div>
          </section>
          <form
            onSubmit={handleSubmit}
            style={{
              background: "var(--aurora-panel-medium)",
              border: "1px solid var(--aurora-border-default)",
              borderRadius: "var(--radius-lg)",
              padding: "1.25rem",
            }}
          >
            <p
              style={{
                color: "var(--aurora-text-muted)",
                fontSize: "0.75rem",
                fontWeight: 600,
                textTransform: "uppercase",
                letterSpacing: "0.05em",
                marginBottom: "0.5rem",
              }}
            >
              {action.label}
            </p>
            <p
              style={{
                color: "var(--aurora-text-muted)",
                fontSize: "0.8rem",
                marginBottom: "1rem",
              }}
            >
              {action.description}
            </p>

            {action.params.length > 0 ? (
              <div className="space-y-3 mb-4">
                {action.params.map((param) => (
                  <div key={param.name}>
                    <label
                      htmlFor={param.name}
                      style={{
                        display: "block",
                        color: "var(--aurora-text-muted)",
                        fontSize: "0.75rem",
                        marginBottom: "0.25rem",
                        fontWeight: 500,
                      }}
                    >
                      {param.label}
                      {param.required && (
                        <span style={{ color: "var(--aurora-error)", marginLeft: "0.25rem" }}>
                          *
                        </span>
                      )}
                    </label>
                    <ParamInput
                      id={param.name}
                      type={param.type}
                      placeholder={param.placeholder}
                      options={param.options}
                      value={paramValues[param.name] ?? ""}
                      onChange={(value) =>
                        setParamValues((prev) => ({ ...prev, [param.name]: value }))
                      }
                      required={param.required}
                    />
                  </div>
                ))}
              </div>
            ) : (
              <p
                style={{
                  color: "var(--aurora-text-muted)",
                  fontSize: "0.8rem",
                  marginBottom: "1rem",
                }}
              >
                No parameters required.
              </p>
            )}

            <SubmitButton
              loading={loading}
              disabled={
                capability === "checking" ||
                !scopeAllowed(capability, action.scope) ||
                (Boolean(action.destructive) && !destructiveAllowed)
              }
            />
            {action.destructive && !destructiveAllowed && (
              <p className="mt-2 text-xs" style={{ color: "var(--aurora-warn)" }}>
                This REST action requires the server destructive-action override or MCP
                confirmation.
              </p>
            )}
          </form>

          {response !== null && <ResponsePanel response={response} isError={isError} />}

          {/* Request preview */}
          <div
            style={{
              background: "var(--aurora-panel-medium)",
              border: "1px solid var(--aurora-border-default)",
              borderRadius: "var(--radius-lg)",
              padding: "1rem",
            }}
          >
            <p
              style={{
                color: "var(--aurora-text-muted)",
                fontSize: "0.75rem",
                fontWeight: 600,
                textTransform: "uppercase",
                letterSpacing: "0.05em",
                marginBottom: "0.5rem",
              }}
            >
              Request Preview
            </p>
            <pre
              style={{
                color: "var(--aurora-text-muted)",
                fontFamily: "var(--aurora-font-mono)",
                fontSize: "0.75rem",
                margin: 0,
                whiteSpace: "pre-wrap",
              }}
            >
              {`POST ${WEB_APP_CONFIG.restEndpoint}\nContent-Type: application/json\n\n${JSON.stringify(requestPreview, null, 2)}`}
            </pre>
          </div>
        </div>
      </div>
    </div>
  );
}

function credentialMessage(state: CapabilityState): string {
  switch (state) {
    case "checking":
      return "Checking the server capabilities for this browser session…";
    case "read":
      return "The current credential permits read actions. Write actions remain disabled.";
    case "write":
      return "The current session permits read and write actions.";
    case "expired":
      return "The stored credential is expired or invalid. Save a valid credential to continue.";
    case "unavailable":
      return "Server capabilities could not be verified. Protected actions remain disabled until the server is reachable.";
    default:
      return "Protected actions are disabled. Credentials are kept only in memory and cleared on reload.";
  }
}

function buildParams(
  params: readonly ActionParam[],
  values: Record<string, string>,
): Record<string, unknown> {
  const payload: Record<string, unknown> = {};

  for (const param of params) {
    const raw = values[param.name] ?? "";
    if (!raw.trim()) continue;

    if (param.type === "checkbox") {
      payload[param.name] = raw === "true";
    } else if (param.type === "number") {
      payload[param.name] = Number(raw);
    } else if (param.type === "string-list") {
      payload[param.name] = raw
        .split(",")
        .map((part) => part.trim())
        .filter(Boolean);
    } else {
      payload[param.name] = raw.trim();
    }
  }

  return payload;
}
