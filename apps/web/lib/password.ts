// SHA-256-hex the plaintext password before sending it to the Rust
// server. The server treats this hex as the secret and runs it
// through argon2id at rest (see crates/tl-server/src/auth_user.rs).
// SHA-256 alone is not a KDF — this hop only avoids shipping the raw
// password over the wire and into the server's request logs.
export async function sha256Hex(password: string): Promise<string> {
  const data = new TextEncoder().encode(password);
  const digest = await crypto.subtle.digest('SHA-256', data);
  return Array.from(new Uint8Array(digest))
    .map((b) => b.toString(16).padStart(2, '0'))
    .join('');
}
