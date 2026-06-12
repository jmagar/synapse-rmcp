# apps/web

Operator dashboard and interactive tool runner for Synapse2. Built with Next.js 16 (static export), React 19, Tailwind CSS 4, Biome, and the Aurora design system.

## What it is

A static web UI served by the Rust binary alongside the MCP API. Synapse2 exposes API + CLI + MCP + Web for local Flux and Scout workflows.

Three pages:

- **Dashboard** (`/`) — Server health (10s polling), status cards, quick action buttons, activity feed
- **Tool Runner** (`/tools/`) — Select an action, fill in parameters, see the request preview and live JSON response
- **API Explorer** (`/api/`) — Endpoint reference, surface parity table (MCP / REST / CLI), and cURL examples for REST-capable actions

## Stack

| Layer | Choice |
|---|---|
| Framework | Next.js 16 (App Router, static export) |
| Runtime | React 19 |
| Language | TypeScript 6 (strict) |
| Styles | Tailwind CSS 4 + Aurora design tokens |
| Components | shadcn/ui scaffolding over Radix UI primitives |
| Icons | shadcn configured for lucide; add lucide-react when introducing icons |
| Fonts | Manrope (display), Inter (sans), JetBrains Mono (mono) |

## Dev commands

```bash
pnpm dev        # dev server at http://localhost:3000
pnpm build      # static export -> out/
pnpm start      # serve the built out/ directory
pnpm lint       # Biome lint
pnpm check      # Biome lint + format check
pnpm typecheck  # TypeScript type check
pnpm test       # Vitest contract tests
pnpm validate   # Biome check + typecheck + tests + static build
```

## How it connects to the backend

All API calls go through `lib/api.ts`. Service names, endpoints, and action metadata live in `lib/template.ts` so the web UI has one obvious place to track the generated Synapse2 contract.

By default, the base URL is empty (relative) — the Rust server serves both the static files and the API from the same origin, so no CORS configuration is needed. For local `pnpm dev` against a separately running backend, copy `.env.example` to `.env.local` and set `NEXT_PUBLIC_SYNAPSE2_API_BASE_URL` (for example, `http://localhost:40080`).

Every action is dispatched as:

```
POST /v1/synapse2
{ "action": "<action>", "params": { ... } }
```

`callAction` wraps the fetch call with typed `ApiResponse<T>` returns. Health and status use `GET /health` and `GET /status`.

## Design system (Aurora)

Dark mode is forced (`<html className="dark">`). All colors are CSS custom properties — never hardcoded hex values.

**Token layers** (defined in `components/aurora.css`):

| Category | Examples |
|---|---|
| Surfaces | `--aurora-page-bg`, `--aurora-panel-medium`, `--aurora-control-surface` |
| Borders | `--aurora-border-default`, `--aurora-border-strong` |
| Text | `--aurora-text-primary`, `--aurora-text-muted` |
| Accents | `--aurora-accent-*` (cyan), `--aurora-accent-pink*` (rose) |
| Status | `--aurora-success`, `--aurora-warn`, `--aurora-error`, `--aurora-info` |
| Radii | `--aurora-radius-1` (14px), `--aurora-radius-2` (18px), `--aurora-radius-3` (22px) |

Aurora tokens are bridged to shadcn's `--primary`, `--card`, `--destructive` aliases in `globals.css`.

**Adding a component:**

```bash
pnpm dlx shadcn@latest add @aurora/aurora-dialog
pnpm dlx shadcn@latest add @aurora/aurora-data-table
```

Components land in `components/ui/`. Use CVA (`class-variance-authority`) for variants and `cn()` from `lib/utils.ts` for className construction.

## File structure

```
apps/web/
├── app/
│   ├── layout.tsx        # Root layout — nav, forced dark mode, font variables
│   ├── page.tsx          # Dashboard (client component, polling)
│   ├── tools/page.tsx    # Tool Runner
│   ├── api/page.tsx      # API Explorer (static)
│   └── globals.css       # Tailwind import + Aurora token bridge + @theme
├── components/
│   ├── aurora.css        # Aurora token definitions (dark + light)
│   └── ui/               # Aurora/shadcn components
├── lib/
│   ├── api.ts            # Typed REST client
│   ├── template.ts       # Branding, endpoints, action metadata
│   └── utils.ts          # cn() helper
├── components.json       # shadcn config — @aurora registry
├── next.config.ts        # Static export, output: "export"
└── tsconfig.json         # Path aliases (@/* → ./*), strict mode
```

## Constraints

- **Static export only** — no server actions, API routes, or streaming. `output: "export"` in `next.config.ts`.
- **Client components only** for interactive pages — use `"use client"` and React hooks.
- **All colors via CSS custom properties** — never write a raw hex value in a component.
- **All API calls through `lib/api.ts`** — don't fetch directly in components.
- **`cn()` for classNames** — never string concatenation.

## Contract maintenance

`lib/template.ts` must stay aligned with `docs/generated/openapi.json`. The Vitest contract test compares the web action list against the generated OpenAPI `ActionName` enum and `x-template.rest_actions` metadata.

When Synapse2 adds or removes REST actions:

1. Regenerate the OpenAPI document.
2. Update `ACTIONS` in `lib/template.ts` with action descriptions, parameters, scopes, examples, and sample responses.
3. Update dashboard/API examples if the quick actions or parity examples should change.
4. Run `pnpm validate`.
