use std::collections::HashSet;

use sha2::{Digest, Sha256};
use uuid::Uuid;

pub(super) fn normalize_server_slug(value: &str) -> Result<String, &'static str> {
    let value = value.trim().to_ascii_lowercase();
    if value.is_empty()
        || value.len() > 40
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
        || value.starts_with('-')
        || value.ends_with('-')
    {
        return Err("server_slug must be 1-40 lowercase letters, numbers, or hyphens");
    }
    Ok(value)
}

pub(super) fn public_tool_names(
    server_slug: &str,
    connection_id: Uuid,
    upstream_names: &[String],
) -> Result<Vec<String>, &'static str> {
    let mut plain = Vec::with_capacity(upstream_names.len());
    for upstream in upstream_names {
        let normalized = normalize_upstream_name(upstream);
        if normalized.is_empty() {
            return Err("upstream tool name cannot normalize to an empty public name");
        }
        plain.push(format!("{server_slug}__{normalized}"));
    }
    let colliding = plain.iter().fold(
        std::collections::HashMap::<String, usize>::new(),
        |mut counts, name| {
            *counts.entry(name.clone()).or_default() += 1;
            counts
        },
    );
    let mut result = Vec::with_capacity(plain.len());
    let mut seen = HashSet::new();
    for (index, base) in plain.into_iter().enumerate() {
        let needs_hash = base.len() > 128 || colliding.get(base.as_str()).copied().unwrap_or(0) > 1;
        let candidate = if needs_hash {
            let suffix = alias_suffix(connection_id, &upstream_names[index]);
            let prefix = truncate_utf8(&base, 119)
                .trim_end_matches(['.', '-', '_'])
                .to_string();
            format!("{prefix}_{suffix}")
        } else {
            base
        };
        if candidate.is_empty() || candidate.len() > 128 || !seen.insert(candidate.clone()) {
            return Err("upstream tool names do not produce unique public names");
        }
        result.push(candidate);
    }
    Ok(result)
}

fn normalize_upstream_name(value: &str) -> String {
    let mut output = String::new();
    let mut previous_underscore = false;
    for character in value.chars() {
        let accepted = character.is_ascii_alphanumeric() || matches!(character, '_' | '.' | '-');
        let next = if accepted { character } else { '_' };
        if next == '_' && previous_underscore {
            continue;
        }
        previous_underscore = next == '_';
        output.push(next);
    }
    output.trim_matches(['.', '-', '_']).to_string()
}

fn alias_suffix(connection_id: Uuid, upstream_name: &str) -> String {
    let mut hash = Sha256::new();
    hash.update(upstream_name.as_bytes());
    hash.update(connection_id.as_bytes());
    hash.finalize()
        .iter()
        .take(4)
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn truncate_utf8(value: &str, max_bytes: usize) -> &str {
    if value.len() <= max_bytes {
        return value;
    }
    let mut end = max_bytes;
    while !value.is_char_boundary(end) {
        end -= 1;
    }
    &value[..end]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collisions_receive_stable_suffixes() {
        let id = Uuid::nil();
        let values =
            public_tool_names("github", id, &["repo create".into(), "repo@create".into()]).unwrap();
        assert_eq!(values.len(), 2);
        assert_ne!(values[0], values[1]);
        assert!(values.iter().all(|value| value.len() <= 128));
    }
}
