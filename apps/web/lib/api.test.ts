import { afterEach, describe, expect, it, vi } from "vitest";
import {
  apiFetch,
  callAction,
  clearBearerToken,
  getActivity,
  getBearerToken,
  getCapabilities,
  parseJsonBody,
  setBearerToken,
} from "./api";

describe("apiFetch", () => {
  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it("returns parsed JSON for successful responses", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn(async () => new Response(JSON.stringify({ status: "ok" }), { status: 200 })),
    );

    await expect(apiFetch<{ status: string }>("/health")).resolves.toEqual({
      data: { status: "ok" },
    });
  });

  it("uses structured API error messages when available", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn(async () => new Response(JSON.stringify({ error: "forbidden" }), { status: 403 })),
    );

    await expect(apiFetch("/v1/synapse2")).resolves.toEqual({ error: "forbidden", status: 403 });
  });

  it("preserves HTTP status when an error body has no error field", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn(async () => new Response("Bad gateway", { status: 502 })),
    );

    await expect(apiFetch("/v1/synapse2")).resolves.toEqual({ error: "HTTP 502", status: 502 });
  });

  it("normalizes thrown fetch failures", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn(async () => {
        throw new Error("connection refused");
      }),
    );

    await expect(apiFetch("/health")).resolves.toEqual({ error: "connection refused" });
  });
});

describe("parseJsonBody", () => {
  it("handles empty bodies", () => {
    expect(parseJsonBody("")).toEqual({});
    expect(parseJsonBody("   ")).toEqual({});
  });

  it("returns non-JSON text unchanged", () => {
    expect(parseJsonBody("not json")).toBe("not json");
  });
});

describe("browser credential storage", () => {
  afterEach(clearBearerToken);

  it("keeps credentials only in module memory", () => {
    const storage = { getItem: vi.fn(), setItem: vi.fn(), removeItem: vi.fn() };
    vi.stubGlobal("window", { sessionStorage: storage });
    setBearerToken(" secret ");
    expect(getBearerToken()).toBe("secret");
    expect(storage.getItem).not.toHaveBeenCalled();
    expect(storage.setItem).not.toHaveBeenCalled();
    expect(storage.removeItem).not.toHaveBeenCalled();
  });
});

describe("activity API", () => {
  afterEach(() => {
    clearBearerToken();
    vi.unstubAllGlobals();
  });

  it("uses the in-memory bearer credential for the shared audit stream", async () => {
    const fetchMock = vi.fn(
      async () => new Response(JSON.stringify({ events: [] }), { status: 200 }),
    );
    vi.stubGlobal("fetch", fetchMock);
    setBearerToken("read-token");

    await getActivity();

    expect(fetchMock).toHaveBeenCalledWith(
      "/activity",
      expect.objectContaining({ headers: { Authorization: "Bearer read-token" } }),
    );
  });

  it.each([
    ["missing events", {}],
    ["invalid event fields", { events: [{ sequence: "1" }] }],
  ])("rejects %s in a successful response", async (_label, payload) => {
    vi.stubGlobal(
      "fetch",
      vi.fn(async () => new Response(JSON.stringify(payload), { status: 200 })),
    );

    await expect(getActivity()).resolves.toEqual({
      error: "Invalid response payload",
      status: 200,
    });
  });

  it("rejects a successful non-JSON response", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn(async () => new Response("not json", { status: 200 })),
    );

    await expect(getActivity()).resolves.toEqual({
      error: "Invalid response payload",
      status: 200,
    });
  });
});

describe("request lifecycle and capabilities API", () => {
  afterEach(() => {
    clearBearerToken();
    vi.unstubAllGlobals();
  });

  it("passes abort ownership to action fetches", async () => {
    const fetchMock = vi.fn(async () => new Response("{}", { status: 200 }));
    vi.stubGlobal("fetch", fetchMock);
    const controller = new AbortController();

    await callAction("help", {}, controller.signal);

    expect(fetchMock).toHaveBeenCalledWith(
      "/v1/synapse2",
      expect.objectContaining({ signal: controller.signal }),
    );
  });

  it("uses the current bearer credential for capability discovery", async () => {
    const fetchMock = vi.fn(
      async () =>
        new Response(JSON.stringify({ scopes: ["synapse:read"], destructive_allowed: false }), {
          status: 200,
        }),
    );
    vi.stubGlobal("fetch", fetchMock);
    setBearerToken("read-token");

    await getCapabilities();

    expect(fetchMock).toHaveBeenCalledWith(
      "/capabilities",
      expect.objectContaining({ headers: { Authorization: "Bearer read-token" } }),
    );
  });

  it.each([
    ["missing scopes", { destructive_allowed: false }],
    ["non-string scopes", { scopes: [42], destructive_allowed: false }],
    ["missing destructive flag", { scopes: ["synapse:read"] }],
  ])("rejects capabilities with %s", async (_label, payload) => {
    vi.stubGlobal(
      "fetch",
      vi.fn(async () => new Response(JSON.stringify(payload), { status: 200 })),
    );

    await expect(getCapabilities()).resolves.toEqual({
      error: "Invalid response payload",
      status: 200,
    });
  });
});
