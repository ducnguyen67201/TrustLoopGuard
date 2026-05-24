export type SearchParamValue = string | string[] | undefined;

export function readParam(value: SearchParamValue): string | null {
  return Array.isArray(value) ? (value[0] ?? null) : (value ?? null);
}

export function readWorkspaceSlug(searchParams: { workspace?: SearchParamValue }): string | null {
  return readParam(searchParams.workspace);
}
