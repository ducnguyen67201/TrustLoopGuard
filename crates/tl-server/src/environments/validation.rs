use super::EnvironmentStoreError;

pub(super) fn validate_slug(slug: &str) -> Result<(), EnvironmentStoreError> {
    let valid = !slug.is_empty()
        && slug.len() <= 64
        && slug
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');

    if valid {
        Ok(())
    } else {
        Err(EnvironmentStoreError::Validation(
            "environment slug must use lowercase letters, digits, and hyphens".into(),
        ))
    }
}

pub(super) fn validate_name(name: &str) -> Result<(), EnvironmentStoreError> {
    if name.trim().is_empty() {
        Err(EnvironmentStoreError::Validation(
            "environment name is required".into(),
        ))
    } else {
        Ok(())
    }
}

pub(super) fn clean_optional(value: String) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}
