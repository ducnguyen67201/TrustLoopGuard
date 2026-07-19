use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use ring::aead::{Aad, LessSafeKey, Nonce, UnboundKey, AES_256_GCM, NONCE_LEN};
use ring::rand::{SecureRandom, SystemRandom};
use sha2::{Digest, Sha256};

/// Derive the AES-256-GCM seal key from env at startup.
///
/// Requires `TL_GATEWAY_CREDENTIAL_KEY`. Falls back to `TL_API_KEY` with a
/// warning for development compatibility. Panics if neither is set unless the
/// explicit `TL_GATEWAY_ALLOW_INSECURE_DEV_KEY` override is enabled.
pub fn build_seal_key() -> [u8; 32] {
    let allow_insecure_dev_key = std::env::var("TL_GATEWAY_ALLOW_INSECURE_DEV_KEY")
        .map(|value| value == "1" || value.eq_ignore_ascii_case("true"))
        .unwrap_or(false);
    seal_key_material(
        std::env::var("TL_GATEWAY_CREDENTIAL_KEY").ok(),
        std::env::var("TL_API_KEY").ok(),
        allow_insecure_dev_key,
    )
    .unwrap_or_else(|message| panic!("{message}"))
}

fn seal_key_material(
    gateway_secret: Option<String>,
    api_key: Option<String>,
    allow_insecure_dev_key: bool,
) -> Result<[u8; 32], String> {
    if let Some(secret) = gateway_secret.filter(|value| !value.trim().is_empty()) {
        return Ok(Sha256::digest(secret.as_bytes()).into());
    }
    if let Some(secret) = api_key.filter(|value| !value.trim().is_empty()) {
        tracing::warn!(
            "TL_GATEWAY_CREDENTIAL_KEY is not set; \
             falling back to TL_API_KEY for gateway credential encryption. \
             Set TL_GATEWAY_CREDENTIAL_KEY to a dedicated 32-byte secret before deploying."
        );
        return Ok(Sha256::digest(secret.as_bytes()).into());
    }
    if allow_insecure_dev_key {
        tracing::error!(
            "SECURITY: TL_GATEWAY_ALLOW_INSECURE_DEV_KEY enabled. \
             Using an insecure dev-only gateway credential key."
        );
        return Ok(Sha256::digest(b"trustloopguard-local-gateway-key").into());
    }
    Err(
        "TL_GATEWAY_CREDENTIAL_KEY must be set before gateway provider credentials can be sealed"
            .to_string(),
    )
}

pub(crate) fn seal_provider_key(provider_key: &str, seal_key: &[u8; 32]) -> String {
    let unbound = UnboundKey::new(&AES_256_GCM, seal_key).expect("valid AES-256-GCM key");
    let sealing_key = LessSafeKey::new(unbound);
    let rng = SystemRandom::new();
    let mut nonce_bytes = [0_u8; NONCE_LEN];
    rng.fill(&mut nonce_bytes)
        .expect("system random available for gateway credential sealing");
    let nonce = Nonce::assume_unique_for_key(nonce_bytes);
    let mut buffer = provider_key.as_bytes().to_vec();
    sealing_key
        .seal_in_place_append_tag(nonce, Aad::empty(), &mut buffer)
        .expect("gateway credential seal succeeds");

    let mut sealed = nonce_bytes.to_vec();
    sealed.extend(buffer);
    format!("tlgw1_{}", URL_SAFE_NO_PAD.encode(sealed))
}

pub(crate) fn unseal_provider_key(ciphertext: &str, seal_key: &[u8; 32]) -> Result<String, String> {
    let encoded = ciphertext
        .strip_prefix("tlgw1_")
        .ok_or_else(|| "provider credential has unsupported seal format".to_string())?;
    let sealed = URL_SAFE_NO_PAD
        .decode(encoded)
        .map_err(|e| format!("provider credential decode failed: {e}"))?;
    if sealed.len() <= NONCE_LEN {
        return Err("provider credential is truncated".to_string());
    }
    let (nonce_bytes, ciphertext) = sealed.split_at(NONCE_LEN);
    let nonce = Nonce::try_assume_unique_for_key(nonce_bytes)
        .map_err(|_| "provider credential nonce is invalid".to_string())?;
    let unbound = UnboundKey::new(&AES_256_GCM, seal_key)
        .map_err(|_| "gateway credential seal key is invalid".to_string())?;
    let key = LessSafeKey::new(unbound);
    let mut buffer = ciphertext.to_vec();
    let plaintext = key
        .open_in_place(nonce, Aad::empty(), &mut buffer)
        .map_err(|_| "provider credential could not be decrypted".to_string())?;
    String::from_utf8(plaintext.to_vec())
        .map_err(|e| format!("provider credential utf8 decode failed: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seal_key_config_requires_secret_without_explicit_dev_override() {
        let result = seal_key_material(None, None, false);

        assert!(result.is_err());
    }
}
