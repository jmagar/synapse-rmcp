#!/usr/bin/env python3
"""Generate and verify OpenAPI docs for the Synapse2 REST API."""

from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[1]
CARGO = ROOT / "Cargo.toml"
OPERATIONS = ROOT / "src/actions/operations.rs"
OUT = ROOT / "docs/generated/openapi.json"
WEB_METADATA = ROOT / "apps/web/lib/generated-operation-metadata.json"

REST_ENDPOINT = "/v1/synapse2"

# Action-specific param examples. Actions not listed here get an empty params object.
_PARAM_EXAMPLES: dict[str, dict] = {
    "flux.docker.info": {"host": "myhost"},
    "flux.docker.pull": {"host": "myhost", "image": "nginx:latest"},
    "flux.container.list": {"state": "running"},
    "scout.peek": {"host": "myhost", "path": "/etc/hostname"},
    "scout.exec": {"host": "myhost", "path": "/tmp", "command": "hostname"},
}

def read(path: Path) -> str:
    return path.read_text(encoding="utf-8")


def package_version() -> str:
    text = read(CARGO)
    match = re.search(r'^version\s*=\s*"([^"]+)"', text, re.M)
    if not match:
        raise RuntimeError("could not find package version in Cargo.toml")
    return match.group(1)


def operation_entries() -> list[dict[str, Any]]:
    text = read(OPERATIONS)
    pattern = re.compile(
        r'operation!\(\s*"([^"]+)"\s*,\s*(\w+)\s*,\s*"([^"]+)"\s*,\s*'
        r'(?:None|Some\("[^"]+"\))\s*,\s*'
        r'(None|Some\((?:READ_SCOPE|WRITE_SCOPE)\))\s*,\s*'
        r'(true|false)\s*,\s*(Rest|McpOnly)\s*,\s*\[(.*?)\]',
        re.S,
    )
    operations: list[dict[str, Any]] = []
    for name, tool, action, scope_expr, destructive, transport, required_text in pattern.findall(text):
        scope = "public"
        if "READ_SCOPE" in scope_expr:
            scope = "synapse:read"
        elif "WRITE_SCOPE" in scope_expr:
            scope = "synapse:write"
        operations.append(
            {
                "name": name,
                "tool": tool,
                "action": action,
                "scope": scope,
                "transport": transport,
                "destructive": destructive == "true",
                "required_params": re.findall(r'"([^"]+)"', required_text),
            }
        )
    if not operations:
        raise RuntimeError("could not parse OPERATION_SPECS from src/actions/operations.rs")
    return operations


def action_entries() -> list[dict[str, str]]:
    actions: dict[str, str] = {}
    for operation in operation_entries():
        current = actions.get(operation["action"])
        scope = operation["scope"]
        if current is None or scope == "synapse:read" or scope == "public":
            actions[operation["action"]] = scope
    return [{"name": name, "scope": scope} for name, scope in actions.items()]


def mcp_operation_names() -> list[str]:
    return [operation["name"] for operation in operation_entries()]


def rest_actions() -> list[dict[str, Any]]:
    return [
        {
            "name": operation["name"],
            "scope": operation["scope"],
            "transport": "Rest",
            "destructive": operation["destructive"],
            "required_params": operation["required_params"],
        }
        for operation in operation_entries()
        if operation["transport"] == "Rest"
    ]


def schema_ref(name: str) -> dict[str, str]:
    return {"$ref": f"#/components/schemas/{name}"}


def render() -> dict[str, Any]:
    actions = rest_actions()
    action_names = [action["name"] for action in actions]
    version = package_version()
    return {
        "openapi": "3.1.0",
        "info": {
            "title": "Synapse2 REST API",
            "version": version,
            "description": (
                "Generated OpenAPI schema for Synapse2's REST surface. "
                "Loopback deployments have no HTTP auth; non-loopback deployments require "
                "SYNAPSE_MCP_TOKEN or OAuth bearer JWTs. "
                "REST actions require their action-specific scopes when auth is mounted."
            ),
        },
        "servers": [
            {
                "url": "http://localhost:40080",
                "description": "Default local development server",
            }
        ],
        "tags": [
            {"name": "health", "description": "Unauthenticated runtime probes"},
            {"name": "actions", "description": "REST action dispatch"},
        ],
        "paths": {
            "/health": {
                "get": {
                    "tags": ["health"],
                    "summary": "Liveness probe",
                    "operationId": "getHealth",
                    "security": [],
                    "responses": {
                        "200": {
                            "description": "Server is alive",
                            "content": {
                                "application/json": {
                                    "schema": schema_ref("HealthResponse"),
                                    "examples": {"ok": {"value": {"status": "ok"}}},
                                }
                            },
                        }
                    },
                }
            },
            "/ready": {
                "get": {
                    "tags": ["health"],
                    "summary": "Readiness probe",
                    "operationId": "getReady",
                    "security": [],
                    "responses": {
                        "200": {
                            "description": "Host topology is loadable",
                            "content": {"application/json": {"schema": schema_ref("ReadinessResponse")}},
                        },
                        "503": {
                            "description": "Host topology is unavailable or timed out",
                            "content": {"application/json": {"schema": schema_ref("ReadinessResponse")}},
                        },
                    },
                }
            },
            "/openapi.json": {
                "get": {
                    "tags": ["health"],
                    "summary": "OpenAPI schema",
                    "operationId": "getOpenApiSchema",
                    "security": [{"BearerAuth": []}, {}],
                    "responses": {
                        "200": {
                            "description": "Generated OpenAPI schema for the REST surface",
                            "content": {
                                "application/json": {
                                    "schema": {"type": "object", "additionalProperties": True}
                                }
                            },
                        }
                    },
                }
            },
            "/status": {
                "get": {
                    "tags": ["health"],
                    "summary": "Runtime status",
                    "operationId": "getStatus",
                    "security": [],
                    "responses": {
                        "200": {
                            "description": "Runtime status with secrets redacted",
                            "content": {"application/json": {"schema": schema_ref("StatusResponse")}},
                        },
                        "500": {"$ref": "#/components/responses/InternalError"},
                    },
                }
            },
            REST_ENDPOINT: {
                "post": {
                    "tags": ["actions"],
                    "summary": "Dispatch a REST action",
                    "description": (
                        "Thin REST shim over the shared service layer. MCP-only actions are "
                        "not exposed here. Current REST actions: " + ", ".join(action_names) + ". "
                        "When auth is mounted, each action requires its declared scope; "
                        "synapse:write satisfies synapse:read."
                    ),
                    "operationId": "dispatchSynapse2Action",
                    "security": [{"BearerAuth": []}, {}],
                    "requestBody": {
                        "required": True,
                        "content": {
                            "application/json": {
                                "schema": schema_ref("ActionRequest"),
                                "examples": {
                                    action["name"]: {
                                        "value": {
                                            "action": action["name"],
                                            "params": _PARAM_EXAMPLES.get(action["name"], {}),
                                        }
                                    }
                                    for action in actions
                                },
                            }
                        },
                    },
                    "responses": {
                        "200": {
                            "description": "Action result. Shape depends on the requested action.",
                            "content": {"application/json": {"schema": schema_ref("ActionResponse")}},
                        },
                        "400": {"$ref": "#/components/responses/BadRequest"},
                        "401": {"$ref": "#/components/responses/Unauthorized"},
                        "403": {"$ref": "#/components/responses/Forbidden"},
                        "429": {"$ref": "#/components/responses/TooManyRequests"},
                        "500": {"$ref": "#/components/responses/InternalError"},
                    },
                }
            },
        },
        "components": {
            "securitySchemes": {
                "BearerAuth": {
                    "type": "http",
                    "scheme": "bearer",
                    "bearerFormat": "opaque",
                    "description": "Static bearer token in bearer mode; OAuth mode also uses bearer JWTs. Loopback mode does not require HTTP auth.",
                }
            },
            "schemas": {
                "ActionName": {
                    "type": "string",
                    "enum": action_names,
                    "description": "REST-capable action names served by POST /v1/synapse2.",
                },
                "ActionRequest": {
                    "type": "object",
                    "additionalProperties": False,
                    "required": ["action"],
                    "properties": {
                        "action": schema_ref("ActionName"),
                        "params": {
                            "type": "object",
                            "description": "Action-specific parameters for the selected Synapse2 REST action.",
                            "additionalProperties": True,
                            "default": {},
                        },
                    },
                },
                "ActionResponse": {
                    "type": "object",
                    "description": "Action result. Shape depends on the requested action.",
                    "additionalProperties": True,
                },
                "StatusResponse": {
                    "type": "object",
                    "required": ["status"],
                    "properties": {
                        "status": {"type": "string"},
                        "note": {"type": "string"},
                        "server": {"type": "string"},
                        "version": {"type": "string"},
                        "transport": {"type": "string"},
                    },
                    "additionalProperties": True,
                },
                "HealthResponse": {
                    "type": "object",
                    "required": ["status"],
                    "properties": {"status": {"type": "string", "const": "ok"}},
                    "additionalProperties": False,
                },
                "ReadinessResponse": {
                    "type": "object",
                    "required": ["status"],
                    "properties": {
                        "status": {"type": "string", "enum": ["ready", "not_ready"]},
                        "error": {"type": "string"},
                    },
                    "additionalProperties": False,
                },
                "HelpResponse": {
                    "type": "object",
                    "required": ["actions", "mcp_only_actions", "usage", "examples"],
                    "properties": {
                        "actions": {"type": "array", "items": schema_ref("ActionName")},
                        "mcp_only_actions": {"type": "array", "items": {"type": "string"}},
                        "usage": {"type": "string"},
                        "examples": {"type": "object", "additionalProperties": True},
                    },
                    "additionalProperties": True,
                },
                "ErrorResponse": {
                    "type": "object",
                    "required": ["error"],
                    "properties": {"error": {"type": "string"}},
                    "additionalProperties": False,
                },
                "OverloadResponse": {
                    "type": "object",
                    "required": ["error", "retryable"],
                    "properties": {
                        "error": {"type": "string", "const": "server overloaded"},
                        "retryable": {"type": "boolean", "const": True},
                    },
                    "additionalProperties": False,
                },
                "RestTruncationResponse": {
                    "type": "object",
                    "required": ["truncated", "error", "max_response_bytes", "hint"],
                    "properties": {
                        "truncated": {"type": "boolean", "const": True},
                        "error": {
                            "type": "string",
                            "const": "response exceeded REST response size limit",
                        },
                        "max_response_bytes": {"type": "integer", "minimum": 1},
                        "hint": {"type": "string"},
                    },
                    "additionalProperties": False,
                },
            },
            "responses": {
                "BadRequest": {
                    "description": "Validation error",
                    "content": {"application/json": {"schema": schema_ref("ErrorResponse")}},
                },
                "Unauthorized": {
                    "description": "Missing or invalid authentication",
                    "content": {"application/json": {"schema": schema_ref("ErrorResponse")}},
                },
                "Forbidden": {
                    "description": "Authenticated request lacks the required scope",
                    "content": {"application/json": {"schema": schema_ref("ErrorResponse")}},
                },
                "InternalError": {
                    "description": "Internal server error",
                    "content": {"application/json": {"schema": schema_ref("ErrorResponse")}},
                },
                "TooManyRequests": {
                    "description": "Concurrency limit reached; retry after the indicated delay",
                    "headers": {
                        "Retry-After": {
                            "required": True,
                            "schema": {"type": "integer", "minimum": 1},
                        }
                    },
                    "content": {"application/json": {"schema": schema_ref("OverloadResponse")}},
                },
            },
        },
        "x-template": {
            "source": "scripts/check-openapi.py",
            "action_metadata": "src/actions/operations.rs",
            "rest_actions": action_names,
            "rest_operations": actions,
            "mcp_actions": mcp_operation_names(),
            "mcp_only_actions": [name for name in mcp_operation_names() if name not in action_names],
        },
    }


def canonical_json(value: dict[str, Any]) -> str:
    return json.dumps(value, indent=2, sort_keys=False) + "\n"


def validate_openapi(value: dict[str, Any]) -> list[str]:
    failures: list[str] = []
    if value.get("openapi") != "3.1.0":
        failures.append("OpenAPI version must be 3.1.0")
    for path in ["/health", "/ready", "/openapi.json", "/status", REST_ENDPOINT]:
        if path not in value.get("paths", {}):
            failures.append(f"missing path {path}")
    for path, methods in value.get("paths", {}).items():
        for method, operation in methods.items():
            if not operation.get("operationId"):
                failures.append(f"{method.upper()} {path} is missing operationId")
    action_enum = value.get("components", {}).get("schemas", {}).get("ActionName", {}).get("enum")
    entries = action_entries()
    if len(operation_entries()) != 59:
        failures.append(f"operation registry drifted: expected 59 entries, got {len(operation_entries())}")
    expected = [action["name"] for action in rest_actions()]
    if action_enum != expected:
        failures.append(f"ActionName enum drifted: expected {expected}, got {action_enum}")
    x_template = value.get("x-template", {})
    if x_template.get("rest_actions") != expected:
        failures.append(
            f"x-template rest_actions drifted: expected {expected}, got {x_template.get('rest_actions')}"
        )
    if x_template.get("rest_operations") != rest_actions():
        failures.append("x-template rest_operations drifted from src/actions/operations.rs")
    expected_mcp_actions = mcp_operation_names()
    if x_template.get("mcp_actions") != expected_mcp_actions:
        failures.append("x-template mcp_actions drifted")
    expected_mcp_only = [name for name in expected_mcp_actions if name not in expected]
    if x_template.get("mcp_only_actions") != expected_mcp_only:
        failures.append("x-template mcp_only_actions drifted")
    rest_security = value.get("paths", {}).get(REST_ENDPOINT, {}).get("post", {}).get("security")
    if rest_security != [{"BearerAuth": []}, {}]:
        failures.append(
            f"{REST_ENDPOINT} security must document bearer auth and no-local-auth modes"
        )
    openapi_security = value.get("paths", {}).get("/openapi.json", {}).get("get", {}).get("security")
    if openapi_security != [{"BearerAuth": []}, {}]:
        failures.append("/openapi.json security must document mounted auth and loopback access")
    ready_responses = value.get("paths", {}).get("/ready", {}).get("get", {}).get("responses", {})
    if not {"200", "503"}.issubset(ready_responses):
        failures.append("/ready must document both 200 and 503 responses")
    overload = value.get("paths", {}).get(REST_ENDPOINT, {}).get("post", {}).get("responses", {}).get("429")
    if overload != {"$ref": "#/components/responses/TooManyRequests"}:
        failures.append(f"{REST_ENDPOINT} must document the shared 429 response")
    status_props = (
        value.get("components", {})
        .get("schemas", {})
        .get("StatusResponse", {})
        .get("properties", {})
    )
    if "api_url" in status_props:
        failures.append("StatusResponse must not advertise api_url on the public status schema")
    return failures


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--write", action="store_true", help="Rewrite docs/generated/openapi.json")
    parser.add_argument("--check", action="store_true", help="Fail if generated OpenAPI is stale")
    args = parser.parse_args()
    if not args.write and not args.check:
        args.check = True

    value = render()
    rendered = canonical_json(value)
    web_metadata = canonical_json(value["x-template"])
    failures = validate_openapi(value)

    if args.write:
        OUT.parent.mkdir(parents=True, exist_ok=True)
        OUT.write_text(rendered, encoding="utf-8")
        print(f"wrote {OUT.relative_to(ROOT)}")
        WEB_METADATA.write_text(web_metadata, encoding="utf-8")
        print(f"wrote {WEB_METADATA.relative_to(ROOT)}")

    if args.check:
        if not OUT.exists():
            failures.append("docs/generated/openapi.json is missing; run scripts/check-openapi.py --write")
        elif OUT.read_text(encoding="utf-8") != rendered:
            failures.append("docs/generated/openapi.json is stale; run scripts/check-openapi.py --write")
        if not WEB_METADATA.exists():
            failures.append("web operation metadata is missing; run scripts/check-openapi.py --write")
        elif WEB_METADATA.read_text(encoding="utf-8") != web_metadata:
            failures.append("web operation metadata is stale; run scripts/check-openapi.py --write")

    if failures:
        for failure in failures:
            print(f"FAIL: {failure}", file=sys.stderr)
        return 1
    if args.check:
        print("OpenAPI schema is current")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
