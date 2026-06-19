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

- **Monitor** — `/`, `/runs`, `/analytics`, and `/attacks`.
- **Configure** — `/policies`, `/agents`, and `/knowledge-sources`.

Keep workspace/admin surfaces in the secondary section below the separator. Do not add new primary items as a flat list; choose the existing group that matches the workflow.

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
  actions?: ReactNode;      // primary action(s), right-aligned on md+
  className?: string;
}
```

### When to use it

- The first block of every page, inside the `px-4 lg:px-6` content gutter.
- Exactly one `PageHeader` per page — it owns the page's `<h1>`. Do not also render an `<h1>`/`<h2>` page title in a `Card`.
- The shared `PageShell` in `components/workspace/ManagementPages.tsx` already wraps `PageHeader`; management pages get it for free.

### Typography

UI text and prose use the `Inter` sans face (the default). Data — IDs, hashes, metrics, code, and verdict labels — uses `font-mono` (IBM Plex Mono); pair numeric columns with `tabular-nums` (or the `.font-data` helper) for stable digits. Do not set monospace on prose.

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

## Verdict badges

The guardrail verdict colors (`--color-allow`, `--color-rewrite`, `--color-block`, `--color-escalate`) are exposed as `Badge` variants: `<Badge variant="block">blocked</Badge>`. Use these instead of hand-mapping verdict strings to colors at call sites, so verdict color stays consistent and legible in both themes.
