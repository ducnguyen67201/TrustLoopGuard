import { z, type ZodType } from 'zod';

export class HttpError extends Error {
  readonly status: number;
  readonly statusText: string;
  readonly body: string;

  constructor(message: string, status: number, statusText: string, body: string) {
    super(message);
    this.name = 'HttpError';
    this.status = status;
    this.statusText = statusText;
    this.body = body;
  }
}

type HttpOptions = Omit<RequestInit, 'body' | 'headers' | 'method' | 'signal'> & {
  headers?: HeadersInit;
  signal?: AbortSignal | null | undefined;
};

type HttpBodyOptions = HttpOptions & {
  contentType?: string | undefined;
};

type HttpBody = BodyInit | Record<string, unknown> | ReadonlyArray<unknown> | null | undefined;

export const http = {
  get<T>(input: string, schema: ZodType<T>, options?: HttpOptions): Promise<T> {
    return request('GET', withWorkspace(input), schema, undefined, options);
  },

  post<T>(
    input: string,
    body: HttpBody,
    schema: ZodType<T>,
    options?: HttpBodyOptions,
  ): Promise<T> {
    return request('POST', withWorkspace(input), schema, body, options);
  },

  patch<T>(
    input: string,
    body: HttpBody,
    schema: ZodType<T>,
    options?: HttpBodyOptions,
  ): Promise<T> {
    return request('PATCH', withWorkspace(input), schema, body, options);
  },

  delete(input: string, options?: HttpOptions): Promise<void> {
    return request('DELETE', withWorkspace(input), undefined, undefined, options);
  },

  withoutWorkspace: {
    get<T>(input: string, schema: ZodType<T>, options?: HttpOptions): Promise<T> {
      return request('GET', input, schema, undefined, options);
    },

    post<T>(
      input: string,
      body: HttpBody,
      schema: ZodType<T>,
      options?: HttpBodyOptions,
    ): Promise<T> {
      return request('POST', input, schema, body, options);
    },

    patch<T>(
      input: string,
      body: HttpBody,
      schema: ZodType<T>,
      options?: HttpBodyOptions,
    ): Promise<T> {
      return request('PATCH', input, schema, body, options);
    },

    delete(input: string, options?: HttpOptions): Promise<void> {
      return request('DELETE', input, undefined, undefined, options);
    },
  },
};

function withWorkspace(input: string): string {
  if (typeof window === 'undefined') return input;
  const workspace = new URLSearchParams(window.location.search).get('workspace');
  if (workspace === null || workspace.trim() === '') return input;
  const separator = input.includes('?') ? '&' : '?';
  return `${input}${separator}workspace=${encodeURIComponent(workspace)}`;
}

async function request<T>(
  method: string,
  input: string,
  schema: ZodType<T> | undefined,
  body: HttpBody,
  options?: HttpBodyOptions,
): Promise<T> {
  const { contentType, headers: optionHeaders, signal, ...requestOptions } = options ?? {};
  const headers = new Headers(optionHeaders);
  const serializedBody = serializeBody(body, contentType, headers);
  const init: RequestInit = {
    ...requestOptions,
    method,
    headers,
  };
  if (signal !== undefined) {
    init.signal = signal;
  }
  if (serializedBody !== undefined) {
    init.body = serializedBody;
  }

  const res = await fetch(input, init);
  if (!res.ok) {
    const text = await res.text().catch(() => '');
    throw new HttpError(
      readErrorMessage(text) || `${res.status} ${res.statusText}`,
      res.status,
      res.statusText,
      text,
    );
  }

  if (res.status === 204 || schema === undefined) {
    return undefined as T;
  }

  const text = await res.text();
  let json: unknown;
  try {
    json = text === '' ? null : JSON.parse(text);
  } catch {
    throw new HttpError('API returned invalid JSON', res.status, res.statusText, text);
  }

  const parsed = schema.safeParse(json);
  if (!parsed.success) {
    throw new HttpError(
      `API response did not match the expected schema: ${z.prettifyError(parsed.error)}`,
      res.status,
      res.statusText,
      text,
    );
  }
  return parsed.data;
}

function serializeBody(
  body: HttpBody,
  contentType: string | undefined,
  headers: Headers,
): BodyInit | undefined {
  if (body === undefined || body === null) return undefined;

  if (contentType !== undefined) {
    if (!headers.has('content-type')) headers.set('content-type', contentType);
    return isBodyInit(body) ? body : JSON.stringify(body);
  }

  if (isBodyInit(body)) return body;

  if (!headers.has('content-type')) headers.set('content-type', 'application/json');
  return JSON.stringify(body);
}

function isBodyInit(body: HttpBody): body is BodyInit {
  if (typeof body === 'string') return true;
  if (typeof FormData !== 'undefined' && body instanceof FormData) return true;
  if (typeof URLSearchParams !== 'undefined' && body instanceof URLSearchParams) return true;
  if (typeof Blob !== 'undefined' && body instanceof Blob) return true;
  if (body instanceof ArrayBuffer) return true;
  if (ArrayBuffer.isView(body)) return true;
  if (typeof ReadableStream !== 'undefined' && body instanceof ReadableStream) return true;
  return false;
}

function readErrorMessage(body: string): string | undefined {
  if (body.trim() === '') return undefined;
  try {
    const parsed = JSON.parse(body) as { error?: unknown; message?: unknown };
    if (typeof parsed.error === 'string') return parsed.error;
    if (typeof parsed.message === 'string') return parsed.message;
  } catch {
    return body;
  }
  return body;
}
