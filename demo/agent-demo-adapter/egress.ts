// The ONLY way the workflow agent makes an outbound call. Default-deny: it will
// only ever really POST to a loopback host, so a demo "real side effect" never
// leaves the machine. A non-loopback or malformed target is refused, not sent.

export interface EgressResult {
  sent: boolean;
  status?: number;
  reason: string;
}

const LOOPBACK_HOSTS = new Set(['127.0.0.1', 'localhost', '::1', '[::1]']);

export async function attemptEgress(url: string, body: string): Promise<EgressResult> {
  let parsed: URL;
  try {
    parsed = new URL(url);
  } catch {
    return { sent: false, reason: `sandbox: invalid egress url (${url})` };
  }

  if (!LOOPBACK_HOSTS.has(parsed.hostname)) {
    // Safety boundary: never contact anything off-machine, even though the agent
    // was instructed (by the injection) to do exactly that.
    return { sent: false, reason: `sandbox: non-loopback egress refused (${parsed.hostname})` };
  }

  try {
    const res = await fetch(url, {
      method: 'POST',
      headers: { 'content-type': 'application/json' },
      body,
      redirect: 'manual', // never follow a redirect off the loopback host we vetted
    });
    // A 3xx from the loopback target could point off-machine; the host check only
    // covered the first hop, so treat any redirect as a refusal.
    if (res.status >= 300 && res.status < 400) {
      return { sent: false, reason: `sandbox: redirect refused (${res.headers.get('location') ?? 'unknown'})` };
    }
    return { sent: true, status: res.status, reason: 'sent' };
  } catch (error) {
    return { sent: false, reason: `egress failed: ${error instanceof Error ? error.message : String(error)}` };
  }
}
