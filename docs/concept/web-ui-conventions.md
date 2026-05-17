# Web UI Conventions

Canonical home for shared web UI primitives and cross-page patterns in `apps/web`. Page-specific UI that no other page is expected to reuse does **not** belong here — only patterns intended for reuse.

This doc covers what the pattern is, its API or contract, when to reach for it, which pages currently adopt it, and the canonical empty/loading/error states. When a new shared pattern lands, extend this doc in the same PR.

For dashboard authentication, see [`web-dashboard-authentication.md`](web-dashboard-authentication.md).

## Sidebar Navigation

The primary sidebar groups runtime monitoring separately from configuration:

- **Monitor** — `/`, `/runs`, and `/analytics`.
- **Configure** — `/policies`, `/agents`, and `/knowledge-sources`.

Keep workspace/admin surfaces in the secondary section below the separator. Do not add new primary items as a flat list; choose the existing group that matches the workflow.

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

- Rows with inline editing, drag-and-drop, or expandable sub-rows. Those use cases live in `components/data-table.tsx` (the TanStack-backed variant) and are out of scope for `DataTable`.
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

## BatchActionBar

`apps/web/components/ui/batch-action-bar.tsx` is the shared action surface for selected table rows. Pair it with `DataTable.selection` and `hooks/use-row-selection`.

Pages own the domain behavior. The shared component only renders the selected count, action buttons, and a clear-selection affordance.

Current adopter:

- `/policies` — enable, disable, and delete selected policies.
- `/api-keys` — revoke selected active API keys.

Use resource-specific API calls behind each action. Do not make `BatchActionBar` know about policies, agents, API keys, or team members.
