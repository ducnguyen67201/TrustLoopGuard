const MIN_USERNAME_LEN: usize = 3;
const MAX_USERNAME_LEN: usize = 64;
/// SHA-256 hex digest is always exactly 64 hex chars.
const SHA256_HEX_LEN: usize = 64;

pub(super) fn validate_username(username: &str) -> Result<(), String> {
    let trimmed = username.trim();
    if trimmed.len() < MIN_USERNAME_LEN {
        return Err(format!(
            "username must be at least {MIN_USERNAME_LEN} characters"
        ));
    }
    if trimmed.len() > MAX_USERNAME_LEN {
        return Err(format!(
            "username must be at most {MAX_USERNAME_LEN} characters"
        ));
    }
    if !trimmed
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == '.')
    {
        return Err("username may only contain a-z, A-Z, 0-9, _, -, .".into());
    }
    Ok(())
}

pub(super) fn validate_password_hex(password: &str) -> Result<(), String> {
    if password.len() != SHA256_HEX_LEN || !password.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err("password must be a SHA-256 hex digest (64 lowercase hex chars)".into());
    }
    Ok(())
}
