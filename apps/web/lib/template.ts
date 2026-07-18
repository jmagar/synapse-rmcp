import operationMetadata from "./generated-operation-metadata.json";

export const WEB_APP_CONFIG = {
  serviceName: "synapse",
  displayName: "Synapse2",
  dashboardTitle: "Synapse2 Operator Dashboard",
  description: "Local Synapse workflows for Flux and Scout operations",
  apiBaseUrl: process.env.NEXT_PUBLIC_SYNAPSE2_API_BASE_URL ?? "",
  restEndpoint: "/v1/synapse2",
  healthEndpoint: "/health",
  statusEndpoint: "/status",
  activityEndpoint: "/activity",
  capabilitiesEndpoint: "/capabilities",
  mcpEndpoint: "/mcp",
} as const;

type ActionParamBase = {
  name: string;
  label: string;
  placeholder?: string;
  required: boolean;
  description: string;
};

export type ActionParam =
  | (ActionParamBase & {
      type: "select";
      options: readonly [string, ...string[]];
    })
  | (ActionParamBase & {
      type: "text" | "number" | "checkbox" | "string-list";
      options?: never;
    });

export type ActionScope = "synapse:read" | "synapse:write" | "public";

export type ActionSpec = {
  id: string;
  label: string;
  description: string;
  scope: ActionScope;
  destructive?: boolean;
  transport: "rest" | "mcp-only";
  params: readonly ActionParam[];
  example: {
    action: string;
    params: Record<string, unknown>;
  };
  response: Record<string, unknown>;
};

type ActionPresentation = Omit<ActionSpec, "scope" | "destructive" | "transport">;

const HOST_PARAM = {
  name: "host",
  label: "Host",
  type: "text",
  placeholder: "myhost",
  required: false,
  description: "Target host. Leave empty to fan out when the action supports it.",
} as const satisfies ActionParam;

const ACTION_PRESENTATIONS = [
  {
    id: "help",
    label: "help",
    description: "Show Synapse2 REST actions and usage documentation.",
    params: [],
    example: { action: "help", params: {} },
    response: {
      actions: [
        "help",
        "flux.docker.info",
        "flux.docker.df",
        "flux.docker.images",
        "flux.docker.networks",
        "flux.docker.volumes",
        "flux.docker.pull",
        "flux.docker.build",
        "flux.docker.rmi",
        "flux.docker.prune",
        "flux.container.list",
        "scout.nodes",
        "scout.peek",
        "scout.exec",
      ],
      usage:
        "Use MCP tools `flux` and `scout`, or CLI commands `synapse flux ...` and `synapse scout ...`.",
    },
  },
  {
    id: "flux.docker.info",
    label: "flux.docker.info",
    description: "Return Docker daemon information for one host or all configured hosts.",
    params: [HOST_PARAM],
    example: { action: "flux.docker.info", params: { host: "myhost" } },
    response: { host: "myhost", server_version: "28.x", containers: 42 },
  },
  {
    id: "flux.docker.df",
    label: "flux.docker.df",
    description: "Show Docker disk usage for images, containers, volumes, and build cache.",
    params: [HOST_PARAM],
    example: { action: "flux.docker.df", params: {} },
    response: { hosts: [{ host: "myhost", layers_size: 123456789 }] },
  },
  {
    id: "flux.docker.images",
    label: "flux.docker.images",
    description: "List Docker images, optionally filtering to dangling images.",
    params: [
      HOST_PARAM,
      {
        name: "dangling_only",
        label: "Dangling only",
        type: "checkbox",
        required: false,
        description: "Only include dangling images.",
      },
    ],
    example: { action: "flux.docker.images", params: {} },
    response: { images: [{ repository: "nginx", tag: "latest", size: 123456789 }] },
  },
  {
    id: "flux.docker.networks",
    label: "flux.docker.networks",
    description: "List Docker networks.",
    params: [HOST_PARAM],
    example: { action: "flux.docker.networks", params: {} },
    response: { networks: [{ name: "bridge", driver: "bridge" }] },
  },
  {
    id: "flux.docker.volumes",
    label: "flux.docker.volumes",
    description: "List Docker volumes.",
    params: [HOST_PARAM],
    example: { action: "flux.docker.volumes", params: {} },
    response: { volumes: [{ name: "app-data", driver: "local" }] },
  },
  {
    id: "flux.docker.pull",
    label: "flux.docker.pull",
    description: "Pull an image on a target host.",
    params: [
      { ...HOST_PARAM, required: true, description: "Host that should pull the image." },
      {
        name: "image",
        label: "Image",
        type: "text",
        placeholder: "nginx:latest",
        required: true,
        description: "Image reference to pull.",
      },
    ],
    example: { action: "flux.docker.pull", params: { host: "myhost", image: "nginx:latest" } },
    response: { host: "myhost", image: "nginx:latest", status: "pulled" },
  },
  {
    id: "flux.docker.build",
    label: "flux.docker.build",
    description: "Build a Docker image from a context on a target host.",
    params: [
      { ...HOST_PARAM, required: true, description: "Host that should run the build." },
      {
        name: "context",
        label: "Context",
        type: "text",
        placeholder: "/srv/app",
        required: true,
        description: "Build context path on the target host.",
      },
      {
        name: "tag",
        label: "Tag",
        type: "text",
        placeholder: "app:latest",
        required: true,
        description: "Image tag to create.",
      },
      {
        name: "dockerfile",
        label: "Dockerfile",
        type: "text",
        placeholder: "Dockerfile",
        required: false,
        description: "Optional Dockerfile path relative to the context.",
      },
      {
        name: "no_cache",
        label: "No cache",
        type: "checkbox",
        required: false,
        description: "Build without using Docker cache.",
      },
    ],
    example: {
      action: "flux.docker.build",
      params: { host: "myhost", context: "/srv/app", tag: "app:latest" },
    },
    response: { host: "myhost", tag: "app:latest", status: "built" },
  },
  {
    id: "flux.docker.rmi",
    label: "flux.docker.rmi",
    description: "Remove a Docker image from a target host.",
    params: [
      { ...HOST_PARAM, required: true, description: "Host containing the image." },
      {
        name: "image",
        label: "Image",
        type: "text",
        placeholder: "nginx:latest",
        required: true,
        description: "Image reference or ID to remove.",
      },
      {
        name: "force",
        label: "Force",
        type: "checkbox",
        required: true,
        description: "Required safety acknowledgement for image removal.",
      },
    ],
    example: {
      action: "flux.docker.rmi",
      params: { host: "myhost", image: "nginx:latest", force: true },
    },
    response: { host: "myhost", image: "nginx:latest", removed: true },
  },
  {
    id: "flux.docker.prune",
    label: "flux.docker.prune",
    description: "Prune unused Docker resources on a target host.",
    params: [
      { ...HOST_PARAM, required: true, description: "Host to prune." },
      {
        name: "prune_target",
        label: "Target",
        type: "select",
        options: ["containers", "images", "volumes", "networks", "buildcache", "all"],
        placeholder: "images",
        required: true,
        description: "Resource class to prune, such as system, images, containers, or volumes.",
      },
      {
        name: "force",
        label: "Force",
        type: "checkbox",
        required: true,
        description: "Required safety acknowledgement for pruning Docker resources.",
      },
    ],
    example: {
      action: "flux.docker.prune",
      params: { host: "myhost", prune_target: "images", force: true },
    },
    response: { host: "myhost", reclaimed_bytes: 123456789 },
  },
  {
    id: "flux.container.list",
    label: "flux.container.list",
    description: "List containers with optional state, name, image, and label filters.",
    params: [
      HOST_PARAM,
      {
        name: "state",
        label: "State",
        type: "text",
        placeholder: "running",
        required: false,
        description: "Container state filter.",
      },
      {
        name: "name_filter",
        label: "Name filter",
        type: "text",
        required: false,
        description: "Filter containers by name.",
      },
      {
        name: "image_filter",
        label: "Image filter",
        type: "text",
        required: false,
        description: "Filter containers by image.",
      },
      {
        name: "label_filter",
        label: "Label filter",
        type: "text",
        required: false,
        description: "Filter containers by label.",
      },
    ],
    example: { action: "flux.container.list", params: { state: "running" } },
    response: { containers: [{ host: "myhost", name: "app", state: "running" }] },
  },
  {
    id: "scout.nodes",
    label: "scout.nodes",
    description: "List configured Scout nodes.",
    params: [],
    example: { action: "scout.nodes", params: {} },
    response: { nodes: [{ host: "myhost", kind: "ssh" }] },
  },
  {
    id: "scout.peek",
    label: "scout.peek",
    description: "Read a file or directory listing from a target host.",
    params: [
      { ...HOST_PARAM, required: true, description: "Host to inspect." },
      {
        name: "path",
        label: "Path",
        type: "text",
        placeholder: "/etc/hostname",
        required: true,
        description: "File or directory path to inspect.",
      },
      {
        name: "tree",
        label: "Tree",
        type: "checkbox",
        required: false,
        description: "Render directories as a tree.",
      },
      {
        name: "depth",
        label: "Depth",
        type: "number",
        placeholder: "3",
        required: false,
        description: "Tree depth, clamped by the server.",
      },
    ],
    example: { action: "scout.peek", params: { host: "myhost", path: "/etc/hostname" } },
    response: { host: "myhost", path: "/etc/hostname", content: "myhost\n" },
  },
  {
    id: "scout.exec",
    label: "scout.exec",
    description: "Run an allowlisted command on a target host.",
    params: [
      { ...HOST_PARAM, required: true, description: "Host to execute on." },
      {
        name: "path",
        label: "Working directory",
        type: "text",
        placeholder: "/tmp",
        required: false,
        description: "Optional working directory.",
      },
      {
        name: "command",
        label: "Command",
        type: "text",
        placeholder: "hostname",
        required: true,
        description: "Allowlisted command binary.",
      },
      {
        name: "args",
        label: "Args",
        type: "string-list",
        placeholder: "-la, /tmp",
        required: false,
        description: "Comma-separated positional arguments.",
      },
      {
        name: "timeout_secs",
        label: "Timeout seconds",
        type: "number",
        placeholder: "30",
        required: false,
        description: "Optional command timeout.",
      },
    ],
    example: {
      action: "scout.exec",
      params: { host: "myhost", path: "/tmp", command: "hostname" },
    },
    response: { host: "myhost", command: "hostname", stdout: "myhost\n", exit_code: 0 },
  },
] as const satisfies readonly ActionPresentation[];

type RestOperationMetadata = {
  name: string;
  scope: ActionScope;
  destructive: boolean;
};

const REST_OPERATION_METADATA = new Map(
  (operationMetadata.rest_operations as RestOperationMetadata[]).map((operation) => [
    operation.name,
    operation,
  ]),
);

export type RestActionId = (typeof ACTION_PRESENTATIONS)[number]["id"];
export type RestAction = Omit<ActionSpec, "id" | "transport"> & {
  id: RestActionId;
  transport: "rest";
};

export const ACTIONS: readonly RestAction[] = ACTION_PRESENTATIONS.map((presentation) => {
  const metadata = REST_OPERATION_METADATA.get(presentation.id);
  if (!metadata) throw new Error(`Missing canonical REST metadata for ${presentation.id}`);
  return {
    ...presentation,
    scope: metadata.scope,
    destructive: metadata.destructive,
    transport: "rest",
  };
});

export const REST_ACTIONS = ACTIONS;
export const DEFAULT_REST_ACTION: RestAction = REST_ACTIONS[0];

export function normalizeApiBaseUrl(apiBaseUrl: string): string {
  return apiBaseUrl.replace(/\/+$/, "");
}

export function endpoint(path: string): string {
  return `${normalizeApiBaseUrl(WEB_APP_CONFIG.apiBaseUrl)}${path}`;
}
