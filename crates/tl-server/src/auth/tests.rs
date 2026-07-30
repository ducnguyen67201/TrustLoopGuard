use super::{token, AuthConfig, EnvError};

#[test]
fn subtle_eq_handles_unequal_lengths() {
    assert!(!token::subtle_eq(b"abc", b"abcd"));
    assert!(!token::subtle_eq(b"abcd", b"abc"));
}

#[test]
fn subtle_eq_matches_equal_bytes() {
    assert!(token::subtle_eq(b"sk-abcdef", b"sk-abcdef"));
}

#[test]
fn subtle_eq_rejects_byte_mismatch() {
    assert!(!token::subtle_eq(b"sk-abcdef", b"sk-abcdex"));
}

#[test]
fn jwt_path_allows_exact_paths_and_subpaths_only() {
    assert!(token::jwt_path_allowed("/v1/team/my-workspaces"));
    assert!(token::jwt_path_allowed("/v1/team/my-workspaces/ws_acme"));
    assert!(token::jwt_path_allowed("/v1/team/invites"));
    assert!(token::jwt_path_allowed("/v1/team/invites/invite_123"));
    assert!(token::jwt_path_allowed("/v1/api-keys"));
    assert!(token::jwt_path_allowed("/v1/api-keys/batch/revoke"));

    assert!(!token::jwt_path_allowed("/v1/team/my-workspaces-admin"));
    assert!(!token::jwt_path_allowed("/v1/team/invites-admin"));
    assert!(!token::jwt_path_allowed("/v1/api-keys-admin"));
    assert!(!token::jwt_path_allowed("/v1/team/members"));
}

#[test]
fn from_env_rejects_missing() {
    std::env::set_var("TL_API_KEY", "");
    let err = AuthConfig::from_env().unwrap_err();
    assert!(matches!(err, EnvError::Empty));
}
