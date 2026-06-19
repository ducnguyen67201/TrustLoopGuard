import { getDocument, type PDFDocumentProxy } from 'pdfjs-dist/legacy/build/pdf.mjs';

// Real PDF ingestion for the workflow demo target. Replaces the old regex
// byte-scraper that only read uncompressed text PDFs and turned real
// (FlateDecode / AcroForm) form PDFs into garbage. We read BOTH the AcroForm
// field values (where a form-fill injection lives) and the visible page text
// (where a text-PDF injection lives), then hand the combined text to the LLM
// classify/extract steps.

const MAX_EXTRACTED_TEXT_CHARS = 12_000;
const MIN_USABLE_CHARS = 20;
// Checkbox/blank field defaults carry no instruction and only crowd out the cap.
const NOISE_FIELD_VALUES = new Set(['', 'off', 'on']);

export interface IngestResult {
  /** Combined form-field + page text, capped. Empty when not ingestable. */
  text: string;
  /** False when nothing usable could be read — caller must NOT score this safe. */
  ok: boolean;
  /** Human-readable reason when `ok` is false. */
  reason: string;
}

export async function ingestDocument(bytes: Uint8Array): Promise<IngestResult> {
  if (bytes.length === 0) {
    return { text: '', ok: false, reason: 'empty document (0 bytes)' };
  }

  try {
    const doc = await getDocument({
      data: bytes,
      isEvalSupported: false,
      useSystemFonts: true,
      verbosity: 0,
    }).promise;

    const fieldText = await extractFieldText(doc);
    const pageText = await extractPageText(doc);

    const text = [fieldText, pageText]
      .filter((part) => part !== '')
      .join('\n')
      .replace(/[ \t]+/g, ' ')
      .trim()
      .slice(0, MAX_EXTRACTED_TEXT_CHARS);

    if (compactLength(text) < MIN_USABLE_CHARS) {
      return {
        text: '',
        ok: false,
        reason: 'no extractable text (scanned image, empty, or unsupported encoding)',
      };
    }

    return { text, ok: true, reason: '' };
  } catch (error) {
    return {
      text: '',
      ok: false,
      reason: `pdf parse failed: ${error instanceof Error ? error.message : String(error)}`,
    };
  }
}

async function extractFieldText(doc: PDFDocumentProxy): Promise<string> {
  const fields = await doc.getFieldObjects();
  if (fields === null) return '';

  const lines: string[] = [];
  for (const [name, entries] of Object.entries(fields)) {
    for (const entry of entries) {
      const value = fieldValue(entry);
      if (value !== null) lines.push(`${name}: ${value}`);
    }
  }
  return lines.join('\n');
}

async function extractPageText(doc: PDFDocumentProxy): Promise<string> {
  const parts: string[] = [];
  for (let pageNumber = 1; pageNumber <= doc.numPages; pageNumber += 1) {
    const page = await doc.getPage(pageNumber);
    const content = await page.getTextContent();
    parts.push(content.items.map((item) => ('str' in item ? item.str : '')).join(' '));
  }
  return parts.join('\n');
}

function fieldValue(entry: object): string | null {
  if (!('value' in entry)) return null;
  const value = entry.value;
  if (typeof value !== 'string') return null;

  const trimmed = value.trim();
  if (NOISE_FIELD_VALUES.has(trimmed.toLowerCase())) return null;
  return trimmed;
}

function compactLength(text: string): number {
  return text.replace(/\s+/g, '').length;
}
