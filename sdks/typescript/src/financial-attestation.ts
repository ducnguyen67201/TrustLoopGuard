import type { CommitFinancialActionRequest } from './generated/CommitFinancialActionRequest';

const PREFIX = 'tlg-financial-execution-attestation.v1';
const encoder = new TextEncoder();

export type UnsignedFinancialExecutionAttestation = Omit<
  CommitFinancialActionRequest,
  'signature'
>;

export function financialExecutionAttestationMessage(
  actionId: string,
  request: UnsignedFinancialExecutionAttestation,
): Uint8Array {
  const fields = [
    request.connector_id,
    actionId,
    request.grant_id,
    request.action_hash,
    request.provider,
    request.provider_reference,
    request.provider_status,
    request.executed_at,
    request.idempotency_key,
    request.provider_proof_sha256,
  ];
  const lines = [PREFIX, ...fields.map((field) => `${encoder.encode(field).byteLength}:${field}`)];
  return encoder.encode(lines.join('\n'));
}

export async function financialProviderProofSha256(proof: string): Promise<string> {
  const digest = await crypto.subtle.digest('SHA-256', arrayBuffer(encoder.encode(proof)));
  return `sha256:${hex(new Uint8Array(digest))}`;
}

export async function signFinancialExecutionAttestation(
  plaintextSecret: string,
  actionId: string,
  request: UnsignedFinancialExecutionAttestation,
): Promise<string> {
  const key = await crypto.subtle.importKey(
    'raw',
    arrayBuffer(decodeBase64Url(plaintextSecret)),
    { name: 'HMAC', hash: 'SHA-256' },
    false,
    ['sign'],
  );
  const signature = await crypto.subtle.sign(
    'HMAC',
    key,
    arrayBuffer(financialExecutionAttestationMessage(actionId, request)),
  );
  return `v1=${encodeBase64Url(new Uint8Array(signature))}`;
}

function decodeBase64Url(value: string): Uint8Array {
  const normalized = value.replace(/-/g, '+').replace(/_/g, '/');
  const padded = normalized.padEnd(Math.ceil(normalized.length / 4) * 4, '=');
  const binary = atob(padded);
  return Uint8Array.from(binary, (character) => character.charCodeAt(0));
}

function encodeBase64Url(value: Uint8Array): string {
  let binary = '';
  for (const byte of value) binary += String.fromCharCode(byte);
  return btoa(binary).replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/g, '');
}

function hex(value: Uint8Array): string {
  return Array.from(value, (byte) => byte.toString(16).padStart(2, '0')).join('');
}

function arrayBuffer(value: Uint8Array): ArrayBuffer {
  const copy = new Uint8Array(value.byteLength);
  copy.set(value);
  return copy.buffer;
}
