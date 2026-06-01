use tl_core::ApiErrorCode;

use super::synthesize_api_error;

#[test]
fn synthesize_unknown_body_uses_status_fallback() {
    let err = synthesize_api_error(503, "");
    assert_eq!(err.code, ApiErrorCode::Unavailable);
    assert!(err.retriable);
}

#[test]
fn synthesize_400_is_not_retriable() {
    let err = synthesize_api_error(400, "bad input");
    assert_eq!(err.code, ApiErrorCode::Invalid);
    assert!(!err.retriable);
    assert_eq!(err.message, "bad input");
}
