import { describe, expect, it } from "vitest";
import { capabilityStateFor, RequestCoordinator, scopeAllowed } from "./request-state";

describe("RequestCoordinator", () => {
  it("rejects rapid double submissions synchronously", () => {
    const requests = new RequestCoordinator();
    expect(requests.begin()).not.toBeNull();
    expect(requests.begin()).toBeNull();
  });

  it("rejects stale completions after the selected action changes", () => {
    const requests = new RequestCoordinator();
    const stale = requests.begin();
    expect(stale).not.toBeNull();
    requests.cancel();
    const current = requests.begin();
    if (!stale || !current) throw new Error("expected request leases");
    expect(requests.isCurrent(stale)).toBe(false);
    expect(requests.isCurrent(current)).toBe(true);
  });

  it("aborts owned fetches during cleanup", () => {
    const requests = new RequestCoordinator();
    const lease = requests.begin();
    requests.cancel();
    expect(lease?.signal.aborted).toBe(true);
  });
});

describe("capabilities", () => {
  it("distinguishes anonymous, read, write, and expired credentials", () => {
    expect(capabilityStateFor(undefined, 401, false)).toBe("anonymous");
    expect(capabilityStateFor(undefined, 401, true)).toBe("expired");
    expect(
      capabilityStateFor({ scopes: ["synapse:read"], destructive_allowed: false }, 200, true),
    ).toBe("read");
    expect(
      capabilityStateFor(
        { scopes: ["synapse:read", "synapse:write"], destructive_allowed: true },
        200,
        true,
      ),
    ).toBe("write");
  });

  it("keeps transient and malformed capability failures distinct from anonymous access", () => {
    expect(capabilityStateFor(undefined, 429, false, "HTTP 429")).toBe("unavailable");
    expect(capabilityStateFor(undefined, 500, true, "HTTP 500")).toBe("unavailable");
    expect(capabilityStateFor(undefined, undefined, false, "connection refused")).toBe(
      "unavailable",
    );
    expect(capabilityStateFor(undefined, 200, false, "Invalid response payload")).toBe(
      "unavailable",
    );
  });

  it("gates actions by their declared scope", () => {
    expect(scopeAllowed("anonymous", "public")).toBe(true);
    expect(scopeAllowed("read", "synapse:read")).toBe(true);
    expect(scopeAllowed("read", "synapse:write")).toBe(false);
    expect(scopeAllowed("write", "synapse:write")).toBe(true);
    expect(scopeAllowed("unavailable", "synapse:read")).toBe(false);
  });
});
