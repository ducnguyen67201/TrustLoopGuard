use sha2::{Digest, Sha256};

const JWT_ALLOWED_PREFIXES: &[&str] = &["/v1/team/my-workspaces", "/v1/api-keys"];

pub(crate) fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut out = String::with_capacity(digest.len() * 2);
    for byte in digest {
        out.push_str(&format!("{byte:02x}"));
    }
    out
}

/// Constant-time byte comparison so 401 latency does not leak the
/// shared prefix of the configured key. Cheap enough for short tokens.
pub(super) fn subtle_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff: u8 = 0;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

pub(super) fn jwt_path_allowed(path: &str) -> bool {
    JWT_ALLOWED_PREFIXES.iter().any(|prefix| {
        path == *prefix
            || path
                .strip_prefix(prefix)
                .is_some_and(|suffix| suffix.starts_with('/'))
    })
}
