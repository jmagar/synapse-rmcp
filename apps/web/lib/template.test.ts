import { readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";
import { ACTIONS, normalizeApiBaseUrl, REST_ACTIONS } from "./template";

type OpenApiActionMetadata = {
  components: {
    schemas: {
      ActionName: {
        enum: string[];
      };
    };
  };
  "x-template": {
    rest_actions: string[];
    mcp_only_actions: string[];
    rest_operations: Array<{
      name: string;
      scope: string;
      destructive: boolean;
      required_params: string[];
    }>;
  };
};

const here = dirname(fileURLToPath(import.meta.url));
const openApi = JSON.parse(
  readFileSync(resolve(here, "../../../docs/generated/openapi.json"), "utf8"),
) as OpenApiActionMetadata;

describe("template action metadata", () => {
  it("keeps REST actions aligned with generated OpenAPI metadata", () => {
    const webRestActions = REST_ACTIONS.map((action) => action.id);
    expect(webRestActions).toEqual(openApi.components.schemas.ActionName.enum);
    expect(webRestActions).toEqual(openApi["x-template"].rest_actions);
  });

  it("keeps scope, destructive, and required-field metadata aligned", () => {
    const normalized = REST_ACTIONS.map((action) => ({
      name: action.id,
      scope: action.scope,
      destructive: Boolean(action.destructive),
      required_params: action.params.filter((param) => param.required).map((param) => param.name),
    }));
    const generated = openApi["x-template"].rest_operations.map(
      ({ name, scope, destructive, required_params }) => ({
        name,
        scope,
        destructive,
        required_params,
      }),
    );
    expect(normalized).toEqual(generated);
  });

  it("reports the actual MCP-only operation set", () => {
    const rest = new Set<string>(REST_ACTIONS.map((action) => action.id));
    expect(openApi["x-template"].mcp_only_actions.length).toBeGreaterThan(0);
    expect(openApi["x-template"].mcp_only_actions.every((action) => !rest.has(action))).toBe(true);
  });

  it("does not duplicate action identifiers", () => {
    const ids = ACTIONS.map((action) => action.id);
    expect(new Set(ids).size).toBe(ids.length);
  });

  it("keeps destructive Docker safety fields required", () => {
    const rmi = ACTIONS.find((action) => action.id === "flux.docker.rmi");
    const prune = ACTIONS.find((action) => action.id === "flux.docker.prune");
    expect(rmi?.params.find((param) => param.name === "force")?.required).toBe(true);
    expect(prune?.params.find((param) => param.name === "prune_target")?.required).toBe(true);
    expect(prune?.params.find((param) => param.name === "force")?.required).toBe(true);
  });
});

describe("normalizeApiBaseUrl", () => {
  it("removes one or more trailing slashes", () => {
    expect(normalizeApiBaseUrl("http://localhost:3100/")).toBe("http://localhost:3100");
    expect(normalizeApiBaseUrl("http://localhost:3100///")).toBe("http://localhost:3100");
  });

  it("preserves empty same-origin configuration", () => {
    expect(normalizeApiBaseUrl("")).toBe("");
  });
});
