const SUBSCRIBE_TIMEOUT_MS = 8_000;

type SubmitOptions = {
  fetcher?: typeof fetch;
  timeoutMs?: number;
};

type FrustrationEvent = {
  type: 'report_frustration';
  category: 'technical';
  severity: 4;
  observation: string;
  visibleEvidence: string;
  currentUrl: string;
  x: number;
  y: number;
  step: 1;
  suggestedDirection: string;
};

type EventInput = {
  currentUrl: string;
  rect: Pick<DOMRect, 'left' | 'top' | 'width' | 'height'>;
  viewport: { width: number; height: number };
};

export async function submitWaitlist(
  payload: Record<string, FormDataEntryValue | string>,
  options: SubmitOptions = {},
) {
  const fetcher = options.fetcher ?? fetch;

  try {
    const response = await fetcher('/api/subscribe', {
      method: 'POST',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify(payload),
      signal: AbortSignal.timeout(options.timeoutMs ?? SUBSCRIBE_TIMEOUT_MS),
    });
    const body = (await response.json().catch(() => ({}))) as { error?: string };
    if (!response.ok) {
      throw new Error(body.error ?? 'Could not subscribe. Try again in a minute.');
    }
  } catch (error) {
    if (
      error instanceof DOMException &&
      (error.name === 'TimeoutError' || error.name === 'AbortError')
    ) {
      throw new Error('The request took too long. Please try again.');
    }
    throw error;
  }
}

export function buildFrustrationEvent(input: EventInput): FrustrationEvent {
  const centerX = input.rect.left + input.rect.width / 2;
  const centerY = input.rect.top + input.rect.height / 2;

  return {
    type: 'report_frustration',
    category: 'technical',
    severity: 4,
    observation: 'Newsletter submission failed and required retry.',
    visibleEvidence: 'The signup form showed an error and made the submit control available again.',
    currentUrl: input.currentUrl,
    x: clampPercent((centerX / Math.max(1, input.viewport.width)) * 100),
    y: clampPercent((centerY / Math.max(1, input.viewport.height)) * 100),
    step: 1,
    suggestedDirection:
      'Time out stalled submissions, preserve the form, and provide a retry path.',
  };
}

export function emitFrustrationEvent(
  event: FrustrationEvent,
  emit: (line: string) => void = (line) => console.info(line),
) {
  emit(`GRANNY_EVENT ${JSON.stringify(event)}`);
}

export function reportSubscribeFrustration(element: HTMLElement) {
  emitFrustrationEvent(
    buildFrustrationEvent({
      currentUrl: window.location.href,
      rect: element.getBoundingClientRect(),
      viewport: { width: window.innerWidth, height: window.innerHeight },
    }),
  );
}

function clampPercent(value: number) {
  return Math.round(Math.min(100, Math.max(0, value)));
}
