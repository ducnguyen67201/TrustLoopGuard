use tl_core::AgentProfile;

pub(super) fn extract_docs(context: &serde_json::Value) -> Vec<String> {
    context
        .get("docs")
        .and_then(serde_json::Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(|value| value.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default()
}

pub(super) fn summarise_profile(profile: &AgentProfile) -> String {
    let mut summary = String::new();
    summary.push_str(&format!("Display name: {}\n", profile.display_name));

    if !profile.scope.in_scope.is_empty() {
        summary.push_str("In scope:\n");
        for item in &profile.scope.in_scope {
            summary.push_str(&format!("- {item}\n"));
        }
    }

    if !profile.knowledge_sources.is_empty() {
        summary.push_str("Knowledge sources:\n");
        for source in &profile.knowledge_sources {
            let kind = format!("{:?}", source.kind).to_lowercase();
            let mut line = format!("- {} ({kind})", source.kb_id);
            if let Some(description) = &source.description {
                line.push_str(&format!(": {description}"));
            }
            if let Some(url) = &source.url {
                line.push_str(&format!(" [{url}]"));
            }
            summary.push_str(&line);
            summary.push('\n');
        }
    }

    summary
}

pub(super) fn bulleted(items: &[String]) -> String {
    if items.is_empty() {
        "(none)".into()
    } else {
        items
            .iter()
            .map(|item| format!("- {item}"))
            .collect::<Vec<_>>()
            .join("\n")
    }
}
