# Web UI Conventions

Canonical home for shared web UI primitives and cross-page patterns in `apps/web`. Page-specific UI that no other page is expected to reuse does **not** belong here — only patterns intended for reuse.

This doc covers what the pattern is, its API or contract, when to reach for it, which pages currently adopt it, and the canonical empty/loading/error states. When a new shared pattern lands, extend this doc in the same PR.

For dashboard authentication, see [`web-dashboard-authentication.md`](web-dashboard-authentication.md).

## Dashboard API Calls

Browser code calls same-origin routes under `apps/web/app/api/*`; it does not call Rust `/v1/*` endpoints directly and it never attaches Rust bearer tokens. The browser's credential is the Auth.js session cookie. The Next API route is responsible for turning that session into the Rust authorization lane described in [`authorization.md`](authorization.md).

Use `apps/web/lib/http.ts` for browser-side API calls:

- Use `http.get/post/patch/delete` for workspace-scoped dashboard data. This preserves the selected `?workspace=...` and `?environment=...` query parameters automatically.
- Use `http.withoutWorkspace.get/post/patch/delete` only for calls that are intentionally not workspace-scoped, such as account or signup flows.
- Do not add page-local `withWorkspace()` helpers. Add new shared URL behavior to `lib/http.ts` instead.
- Do not use raw `fetch('/api/...')` in reusable dashboard data helpers when the shared `http` client can express the call. Raw `fetch` is reserved for page-specific forms or non-JSON/file flows.

Next API routes that proxy Rust must use request-aware server helpers:

- Use `tlClientForRequest(req)` when calling the generated TypeScript SDK for workspace-scoped Rust endpoints.
- Use `rustApiForAuthorizedWorkspace(req, path, init)` when proxying raw Rust HTTP and the route needs workspace authorization.
- Use `rustApiForUser(user, path, init)` for user-scoped routes that do not operate on the currently selected workspace.
- Do not call bare `tlClient()` from `apps/web/app/api/**/route.ts`; it has no request context and will not attach the Rust `Authorization` header on staging or production.

The intended flow is:

```text
UI component
  -> http.*
  -> /api/... same-origin route
  -> tlClientForRequest(req) or rustApiForAuthorizedWorkspace(req, ...)
  -> Rust /v1/... with Authorization
```

Runtime/product pages carry the selected environment in the URL as `environment=<environment_id>`. Same-origin proxy helpers translate that into the trusted Rust `X-TLG-Environment-Id` header; browser code must not set that header directly.

## Sidebar Navigation

The primary sidebar groups runtime monitoring separately from configuration:

- **Monitor** — `/`, `/runs`, `/approvals`, `/financial`, `/analytics`, and `/attacks`.
- **Configure** — `/policies`, `/grants`, `/agents`, `/knowledge-sources`, `/gateway`, and feature-gated `/mcp-access`.

The `/approvals` route is labeled **Authorization**. Activity is its default tab
and lists recent Rust authorization receipts without expanding raw evidence.
Needs approval is the only actionable queue: it shows immutable envelopes and
lets an Owner/Admin deny a request or mint exact/bounded authority. Approval
history is read-only. `/grants` lists and revokes the resulting authority.
Historical human-review events remain analytics-only and cannot resume
execution or mint grants. See [`authorization-kernel.md`](authorization-kernel.md)
for the runtime contract.

Keep workspace/admin surfaces in the secondary section below the separator. Do not add new primary items as a flat list; choose the existing group that matches the workflow.

Workspace rollout flags returned by `GET /v1/team/my-workspaces` control unavailable product areas. A gated navigation item declares its workspace feature in `components/app-sidebar.tsx`; `lib/workspace-features.ts` is the canonical evaluator. The page entry point must apply the same evaluator and call `notFound()` when disabled, so hiding a link never becomes the only gate. `/attacks` uses `isAttacksEnabled`; `/knowledge-sources` uses `isKnowledgeBaseEnabled`; `/mcp-access` uses `isMcpGatewayEnabled`. All three default to disabled for new and existing workspaces. When enabled during rollout, the earlier beta sidebar labels remain until the product marks them generally available.

The sidebar owns the workspace switcher and environment switcher. Runtime/product pages should preserve both URL parameters when linking within the dashboard so policy deployment toggles, runs, traces, and analytics stay scoped to the selected environment.

The top bar (`apps/web/components/site-header.tsx`) renders a breadcrumb trail. Pass `breadcrumbs={[{ label, href? }]}` for nested routes (e.g. `Runs / <run id>`) so users can navigate back to the parent; the last crumb is the current page and is not linked. With no `breadcrumbs`, it falls back to a single crumb showing the page title.

## PageHeader

`apps/web/components/ui/page-header.tsx` is the single in-content page header. Every dashboard page opens with it so heading rhythm, the primary action position, and spacing are identical everywhere.

### API

```ts
interface PageHeaderProps {
  eyebrow?: ReactNode;      // short context line above the title (e.g. workspace name)
  title: ReactNode;         // rendered as the page's <h1>
  description?: ReactNode;  // sentence-length explanation under the title
  descriptionClassName?: string; // page-specific description measure/typography
  help?: ReactNode;         // optional inline help beside the title — typically an <InfoHint>
  actions?: ReactNode;      // primary action(s), right-aligned on md+
  className?: string;
}
```

Use `description` for the plain-language "what is this page for" sentence (write it
for a non-technical teammate, not an operator who already knows the jargon). Reach
for `help` only when a single word in the title needs defining — pass
`<InfoHint term="…" />` (see below) rather than lengthening the description.
Descriptions use `max-w-prose` by default. Use `descriptionClassName` only when a
page needs a different readable measure, while leaving the shared default intact.

### When to use it

- The first block of every page, inside the `px-4 lg:px-6` content gutter.
- Exactly one `PageHeader` per page — it owns the page's `<h1>`. Do not also render an `<h1>`/`<h2>` page title in a `Card`.
- The shared `PageShell` in `components/workspace/ManagementPages.tsx` already wraps `PageHeader`; management pages get it for free.

### Typography

UI text and prose use the `Inter` sans face (the default). Data — IDs, hashes, metrics, code, and effect labels — uses `font-mono` (IBM Plex Mono); pair numeric columns with `tabular-nums` (or the `.font-data` helper) for stable digits. Do not set monospace on prose.

## DataTable

`apps/web/components/ui/data-table.tsx` is the single component used to render tabular data anywhere in the dashboard. All page-level tables go through it so styling, header treatment, alignment, and empty states stay identical across pages.

### API

```ts
interface DataTableColumn<T> {
  id: string;                       // stable key, used for React keys
  header: ReactNode;                // column heading
  cell: (row: T) => ReactNode;      // cell renderer
  align?: 'left' | 'right' | 'center';
  className?: string;               // applied to both header and cell
  headerClassName?: string;
  cellClassName?: string;
}

interface DataTableProps<T> {
  columns: DataTableColumn<T>[];
  rows: T[];
  getRowKey: (row: T) => string;
  selection?: {
    selectedRowKeys: string[];
    onSelectedRowKeysChange: (keys: string[]) => void;
    getRowCanSelect?: (row: T) => boolean;
  };
  empty?: ReactNode;                // empty-state message
  caption?: ReactNode;              // screen-reader caption
  className?: string;
}
```

The component is generic in `T` — typed against the row shape. Columns are declared as data, not JSX, so every consumer renders rows the same way.

### When to use it

- Any list-of-records view inside a `Card`/`CardContent` on a dashboard page.
- Both static (server-rendered) and dynamic (client-fetched) row collections.
- Batch-edit tables that only need row selection plus page-owned actions.

### When **not** to use it

- Rows with inline editing, drag-and-drop, or expandable sub-rows. Those interactive grid use cases are out of scope for `DataTable`.
- Highly interactive admin grids where filtering, sorting, and pagination must run client-side. Reach for TanStack Table for those.

### Empty state

Always pass an `empty` message tailored to the page (e.g. `"No agents in this workspace yet."`). The default fallback (`"No records yet."`) exists so the component never renders an empty `<tbody>`, but a domain-specific message is expected on every page.

### Current adopters

- `/` — recent decisions (`components/workspace/WorkspaceDashboard.tsx`).
- `/policies` — workspace policies (`components/workspace/PoliciesPageContent.tsx`).
- `/agents`, `/runs`, `/runs/[id]`, `/knowledge-sources`, `/api-keys`, `/team` — management tables in `components/workspace/ManagementPages.tsx`.
- `/approvals` — authorization receipt activity plus pending and historical common approval envelopes (`components/workspace/AuthorizationApprovalsContent.tsx`).
- `/grants` — active and historical common authority (`components/workspace/AuthorizationGrantsContent.tsx`).
- `/mcp-access` — remote server and exact member-agent tool assignment tables (`components/workspace/McpAccessPageContent.tsx`).

When adding a new page with a table, add an entry to this list in the same PR.

### Styling rules

- Do not add a wrapper `<Table>` around `<DataTable>` — it already renders one.
- Do not reach for the raw `components/ui/table` primitives in page code. They are an implementation detail of `DataTable`. New variants of table styling go on `DataTableColumn` (`align`, `className`, etc.), not at call sites.
- Right-align numeric columns with `align: 'right'` and pair with `cellClassName: 'tabular-nums'` for stable digits.
- Wide tables scroll horizontally inside their own container — `Table` wraps itself in a `minmax(0,1fr)` grid so its intrinsic width never propagates to flex/grid ancestors and pushes the page wide. Do not add your own overflow wrappers or `min-w-0` ancestors to "fix" a wide table; it already contains itself on every page.

## BatchActionBar

`apps/web/components/ui/batch-action-bar.tsx` is the shared action surface for selected table rows. Pair it with `DataTable.selection` and `hooks/use-row-selection`.

Pages own the domain behavior. The shared component only renders the selected count, action buttons, and a clear-selection affordance.

Current adopter:

- `/policies` — enable, disable, and delete selected policies.
- `/api-keys` — revoke selected active API keys.

Use resource-specific API calls behind each action. Do not make `BatchActionBar` know about policies, agents, API keys, or team members.

## EmptyState

`apps/web/components/ui/empty-state.tsx` is the shared empty state for panels that loaded successfully but have nothing to show (distinct from `DataTable`'s inline `empty` message, which is for empty table bodies).

### API

```ts
interface EmptyStateProps {
  icon?: ReactNode;
  title: string;          // domain-specific, e.g. "No policies yet"
  description?: string;
  action?: ReactNode;     // a way forward (usually the page's create action)
  className?: string;
}
```

Always give it a domain-specific `title` and, where the user can act, an `action` so an empty surface is never a dead end. Reach for it for whole-card or whole-section emptiness; keep using `DataTable`'s `empty` prop for empty rows inside an otherwise-populated table card.

## CopyBlock

`apps/web/components/onboarding/CopyBlock.tsx` is the shared copy-to-clipboard block: mono content in a quiet bordered panel with an uppercase micro-label and a copy button (icon flips to a check for 2s; clipboard failure falls back to a toast telling the user to copy manually).

### API

```ts
interface CopyBlockProps {
  label: string;          // uppercase micro-label in the header row
  content: string;        // copied verbatim; rendered in a wrapping <pre>
  previewLines?: number;  // lines shown before "Show all" collapses it (default 12)
}
```

Use a small `previewLines` (e.g. 5–6) when the surface only needs to show the *shape* of a snippet, not the whole payload — the copy button still copies the full `content`, so a short preview never truncates what the user pastes.

### When to use it

- Any "here is text the user must paste elsewhere" moment: SDK snippets, AI-assistant prompts, config fragments.
- Not for one-time secrets — the API-key reveal keeps its own `Input` + copy pattern (`CreateApiKeyDialog`) so the secret stays selectable and is never re-rendered.

Content wraps (`whitespace-pre-wrap`) and scrolls inside its own container, so long snippets never widen the page at 360px. Blocks longer than the preview limit collapse by default behind `Show all`; the copy button always copies the full `content`, not the preview.

Current adopters: `/onboarding/connect` (SDK quick-start and assistant prompt).

## OnboardingProgress

`apps/web/components/onboarding/OnboardingProgress.tsx` renders the 3-segment progress rail for the first-run onboarding flow (`/onboarding/workspace` → `/onboarding/connect` → `/onboarding/verify`). Pass `current: 1 | 2 | 3`; segments up to `current` fill with `bg-primary`, the active one carries `aria-current="step"`. Onboarding progress is **derived, not stored**: no workspace → step 1, no API keys → step 2, no traces → step 3, traces exist → done. There is no durable onboarding flag.

## Authorization effect badges

The five canonical effects are exposed as `Badge` variants: `permit`, `transform`, `deny`, `require_approval`, and `defer`. Use these variants instead of mapping colors at call sites. `AuthorizationEffectLegend` is the shared explanatory key for tables and charts.

On `/approvals`, Activity displays Time, Outcome, Domain, Principal, Operation,
and Reason from the environment-scoped receipt list. Rows link to receipt detail
and to a Run when present. Permit is labeled as a policy/authority outcome,
never as a human approval. The empty state says **No authorization activity**.
Needs approval renders the common approval envelope and domain evidence, echoes
`envelope_hash`, and offers scoped approval only when `proposed_scope` exists;
its empty state says **No pending approvals**. Approval history is read-only
context for approved, denied, canceled, and expired approvals. `/grants` creates
typed user-intent grants and revokes active authority. Financial pages show
authorization and execution as separate read-only columns.

## Plain-language help (glossary + InfoHint)

The dashboard is full of domain words a non-technical teammate will not know on sight (effect, grant, policy, agent, gateway, trace). Two pieces keep those explained consistently:

- `apps/web/lib/glossary.ts` — the **one** home for "what does this word mean?". Each entry is `{ label, short }` where `short` is a single jargon-free sentence. Add a term here once; never re-explain the same word with different wording at a call site.
- `apps/web/components/ui/info-hint.tsx` — `<InfoHint term="policy" />` renders a small "?" affordance that reveals the glossary definition on hover/focus. It is a real, keyboard- and touch-accessible button. Pass `term` for a glossary word, or `children` for one-off help text.

### When to use it

- Beside a `PageHeader` title via the `help` prop, when the page's name is itself jargon.
- Next to a table column header or form-field label whose meaning is not obvious (`Effect`, `Severity`, `Scope`).
- Do **not** scatter it on every label — only where a first-time user would genuinely pause. Over-hinting is as noisy as no hints.

## AuthorizationEffectLegend

`apps/web/components/ui/authorization-effect-legend.tsx` renders the canonical effect key from the glossary. Place one legend near a table or chart that displays effect badges; do not repeat one per row or card.

## Gateway setup surface

Gateway exposes two configuration resources: Routes and Providers. A route binds a provider and an
agent; it never asks the operator to choose a second rule set because enabled policies apply
automatically. Route-specific client snippets live below the Routes table instead of in a peer
navigation tab. Empty, loading, and error states must describe those real dependencies and must not
turn authenticated API failures into empty resource counts.

Provider rows expose edit and delete actions. Editing never displays an existing secret: the key
field is optional and only rotates the credential when the operator enters a replacement. Deletion
uses an explicit destructive confirmation that says the provider and stored key are permanently
removed. The API blocks deletion while a route still references the provider.

## Type tokens

Color, spacing, radius, and font-family are defined once in `apps/web/app/globals.css` (Tailwind v4 `@theme`) and consumed as named utilities (`bg-primary`, `p-4`, `rounded-lg`, `font-mono`). Never use raw hex or magic numbers for those — see the rule in `AGENTS.md`.

Tailwind's smallest default size is `text-xs` (12px). For the badge captions, table meta, and uppercase eyebrow labels that run below it, the scale is extended with two named micro sizes plus a label tracking, so call sites stop inventing `text-[10px]`/`text-[11px]` arbitrary classes:

| Utility | Value | Use for |
|---|---|---|
| `text-2xs` | 0.6875rem (11px) | The default micro size — badge text, dense table meta, secondary inline labels. Reach for this first. |
| `text-3xs` | 0.625rem (10px) | Only the smallest chrome — uppercase eyebrow/section labels, legend keys, tag pills. |
| `tracking-label` | 0.14em | Letter-spacing for ALL-CAPS eyebrow/section labels, usually paired with `text-3xs uppercase`. |

Sizes are rem-based, so they scale with the root font and stay overflow-safe. Do not add new arbitrary `text-[…px]` micro classes; if a genuinely new size is needed, add a token here and in `globals.css` rather than at the call site. Two intentional exceptions live off-system and must **not** use these tokens: `app/r/[token]/report-document.tsx` (react-pdf can't read CSS vars) and the VS Code-mimic YAML diff editor.
