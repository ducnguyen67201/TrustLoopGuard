use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};

#[derive(Debug, thiserror::Error)]
pub(super) enum PasswordError {
    #[error("hash: {0}")]
    Hash(String),
    #[error("verify: {0}")]
    Verify(String),
}

/// Argon2id hash of the SHA-256 hex digest sent by the client.
pub(super) fn hash_password(password_hex: &str) -> Result<String, PasswordError> {
    let salt = SaltString::generate(&mut OsRng);
    let argon = Argon2::default();
    argon
        .hash_password(password_hex.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|error| PasswordError::Hash(error.to_string()))
}

/// Verify a candidate SHA-256 hex digest against the stored PHC string.
pub(super) fn verify_password(password_hex: &str, phc: &str) -> Result<bool, PasswordError> {
    let parsed =
        PasswordHash::new(phc).map_err(|error| PasswordError::Verify(error.to_string()))?;
    match Argon2::default().verify_password(password_hex.as_bytes(), &parsed) {
        Ok(()) => Ok(true),
        Err(argon2::password_hash::Error::Password) => Ok(false),
        Err(error) => Err(PasswordError::Verify(error.to_string())),
    }
}
