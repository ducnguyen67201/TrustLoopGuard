//! Static analysis of an imported workflow graph (n8n export today).
//!
//! Classifies nodes into **untrusted sources** (where attacker-controlled data
//! enters: webhooks, form triggers, inbound email, uploaded documents) and
//! **dangerous sinks** (where it can do harm: HTTP egress, outbound email,
//! databases, code execution), then walks the `connections` graph to find
//! injectable `source → sink` paths.
//!
//! The workflow graph *is* the provenance graph: these paths ground the attack
//! planner (what to inject, and at which sink) and double as the static policy
//! seed for the hardening loop's preventive (un-attacked) coverage.
//!
//! Scope (ponytail): ~the high-value n8n node families + a small benign list.
//! Anything else is reported in `unmapped_node_types` (never silently dropped)
//! and treated as a pass-through vertex so it can't hide a real source→sink
//! path. Add an arm when a node family actually shows up unmapped in practice.

use std::collections::{BTreeSet, HashMap, HashSet, VecDeque};

use tl_core::WorkflowPath;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NodeRole {
    Source,
    Sink,
    Neutral,
}

/// Result of analysing one workflow definition.
pub(crate) struct WorkflowAnalysis {
    pub paths: Vec<WorkflowPath>,
    pub unmapped_node_types: Vec<String>,
}

/// Classify a raw node type into a role + coarse category. `None` ⇒ unmapped
/// (unknown to us); the caller records it and treats the node as pass-through.
fn classify(node_type: &str) -> Option<(NodeRole, &'static str)> {
    let lower = node_type.to_ascii_lowercase();
    // n8n types look like `n8n-nodes-base.httpRequest`; key off the leaf.
    let leaf = lower.rsplit('.').next().unwrap_or(lower.as_str());

    // Untrusted entry points. Matchers are deliberately tight — a loose
    // substring (e.g. "form" in "informationExtractor", "code" in "barcode")
    // would invent phantom source→sink paths and spurious findings.
    if leaf == "webhook" {
        return Some((NodeRole::Source, "webhook"));
    }
    if leaf == "form" || leaf.contains("formtrigger") {
        return Some((NodeRole::Source, "form"));
    }
    if leaf.contains("emailread") || leaf.contains("emailimap") || leaf == "imap" {
        return Some((NodeRole::Source, "email_read"));
    }
    if leaf.contains("document")
        || leaf.contains("readpdf")
        || leaf.contains("extractfromfile")
        || leaf.contains("readbinaryfile")
    {
        return Some((NodeRole::Source, "document"));
    }

    // Dangerous operations.
    if leaf.contains("httprequest") {
        return Some((NodeRole::Sink, "http"));
    }
    if leaf.contains("emailsend") || leaf.contains("sendemail") || leaf == "gmail" {
        return Some((NodeRole::Sink, "email_send"));
    }
    if leaf.contains("postgres") || leaf.contains("mysql") || leaf.contains("mongodb") {
        return Some((NodeRole::Sink, "database"));
    }
    if leaf == "code"
        || leaf.contains("executecommand")
        || leaf.contains("functionitem")
        || leaf == "function"
    {
        return Some((NodeRole::Sink, "code_exec"));
    }

    // Recognised-benign control/transform nodes — neutral, not "unmapped".
    const BENIGN: &[&str] = &[
        "set",
        "if",
        "merge",
        "noop",
        "switch",
        "filter",
        "splitout",
        "aggregate",
        "wait",
        "stickynote",
        "manualtrigger",
        "scheduletrigger",
        "cron",
        "start",
        "datetime",
        "limit",
        "sort",
        "itemlists",
        "html",
        "markdown",
        "renamekeys",
    ];
    if BENIGN.contains(&leaf) {
        return Some((NodeRole::Neutral, "neutral"));
    }

    None
}

/// Parse `nodes[]` → (name → raw type). Skips nameless/typeless entries.
fn node_types(definition: &serde_json::Value) -> HashMap<String, String> {
    let mut map = HashMap::new();
    let Some(nodes) = definition.get("nodes").and_then(|v| v.as_array()) else {
        return map;
    };
    for node in nodes {
        let name = node.get("name").and_then(|v| v.as_str());
        let ty = node.get("type").and_then(|v| v.as_str());
        if let (Some(name), Some(ty)) = (name, ty) {
            map.insert(name.to_string(), ty.to_string());
        }
    }
    map
}

/// Build the directed adjacency from n8n `connections`. Every output group
/// (`main`, `ai_tool`, …) is a data edge, so we walk them all.
fn adjacency(definition: &serde_json::Value) -> HashMap<String, Vec<String>> {
    let mut adj: HashMap<String, Vec<String>> = HashMap::new();
    let Some(conns) = definition.get("connections").and_then(|v| v.as_object()) else {
        return adj;
    };
    for (source, outputs) in conns {
        let Some(outputs) = outputs.as_object() else {
            continue;
        };
        for group in outputs.values() {
            // group is an array of arrays of { node, type, index }.
            let Some(slots) = group.as_array() else {
                continue;
            };
            for slot in slots {
                let Some(targets) = slot.as_array() else {
                    continue;
                };
                for target in targets {
                    if let Some(node) = target.get("node").and_then(|v| v.as_str()) {
                        adj.entry(source.clone())
                            .or_default()
                            .push(node.to_string());
                    }
                }
            }
        }
    }
    adj
}

/// Sink node names reachable from `start` through the directed graph.
fn reachable_sinks(
    start: &str,
    adj: &HashMap<String, Vec<String>>,
    sinks: &HashMap<String, &'static str>,
) -> Vec<String> {
    let mut seen: HashSet<&str> = HashSet::new();
    let mut queue: VecDeque<&str> = VecDeque::new();
    let mut found: Vec<String> = Vec::new();
    seen.insert(start);
    queue.push_back(start);
    while let Some(node) = queue.pop_front() {
        if let Some(next) = adj.get(node) {
            for target in next {
                if !seen.insert(target.as_str()) {
                    continue;
                }
                if sinks.contains_key(target) {
                    found.push(target.clone());
                }
                queue.push_back(target.as_str());
            }
        }
    }
    found
}

/// Analyse a workflow definition into injectable `source → sink` paths.
pub(crate) fn analyze(definition: &serde_json::Value) -> WorkflowAnalysis {
    let types = node_types(definition);
    let adj = adjacency(definition);

    let mut sources: Vec<(String, &'static str)> = Vec::new();
    let mut sinks: HashMap<String, &'static str> = HashMap::new();
    let mut unmapped: BTreeSet<String> = BTreeSet::new();
    for (name, ty) in &types {
        match classify(ty) {
            Some((NodeRole::Source, category)) => sources.push((name.clone(), category)),
            Some((NodeRole::Sink, category)) => {
                sinks.insert(name.clone(), category);
            }
            Some((NodeRole::Neutral, _)) => {}
            None => {
                unmapped.insert(ty.clone());
            }
        }
    }

    let mut paths: Vec<WorkflowPath> = Vec::new();
    for (source_node, source_category) in &sources {
        for sink_node in reachable_sinks(source_node, &adj, &sinks) {
            let sink_category = *sinks.get(&sink_node).unwrap_or(&"");
            paths.push(WorkflowPath {
                source_node: source_node.clone(),
                source_type: types.get(source_node).cloned().unwrap_or_default(),
                source_category: source_category.to_string(),
                sink_node: sink_node.clone(),
                sink_type: types.get(&sink_node).cloned().unwrap_or_default(),
                sink_category: sink_category.to_string(),
            });
        }
    }
    // Deterministic order so codegen/tests/UI don't flap on HashMap iteration.
    paths.sort_by(|a, b| {
        (&a.source_node, &a.sink_node, &a.sink_category).cmp(&(
            &b.source_node,
            &b.sink_node,
            &b.sink_category,
        ))
    });

    WorkflowAnalysis {
        paths,
        unmapped_node_types: unmapped.into_iter().collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn finds_source_to_sink_path_through_neutral_node() {
        // Inbox (untrusted email) → Filter (neutral) → Send (http egress).
        let wf = json!({
            "nodes": [
                { "name": "Inbox", "type": "n8n-nodes-base.emailReadImap" },
                { "name": "Filter", "type": "n8n-nodes-base.if" },
                { "name": "Send", "type": "n8n-nodes-base.httpRequest" }
            ],
            "connections": {
                "Inbox": { "main": [[{ "node": "Filter" }]] },
                "Filter": { "main": [[{ "node": "Send" }]] }
            }
        });
        let out = analyze(&wf);
        assert_eq!(out.paths.len(), 1, "expected one injectable path");
        let p = &out.paths[0];
        assert_eq!(p.source_node, "Inbox");
        assert_eq!(p.source_category, "email_read");
        assert_eq!(p.sink_node, "Send");
        assert_eq!(p.sink_category, "http");
        assert!(out.unmapped_node_types.is_empty());
    }

    #[test]
    fn lookalike_node_names_do_not_create_phantom_paths() {
        // "informationExtractor" contains "form"; "barcode" contains "code" —
        // tight matchers must NOT treat them as a source/sink.
        let wf = json!({
            "nodes": [
                { "name": "Extract", "type": "n8n-nodes-langchain.informationExtractor" },
                { "name": "Bar", "type": "n8n-nodes-base.barcode" }
            ],
            "connections": { "Extract": { "main": [[{ "node": "Bar" }]] } }
        });
        let out = analyze(&wf);
        assert!(
            out.paths.is_empty(),
            "look-alike names must not be a source→sink path"
        );
    }

    #[test]
    fn no_path_when_source_does_not_reach_sink() {
        let wf = json!({
            "nodes": [
                { "name": "Hook", "type": "n8n-nodes-base.webhook" },
                { "name": "Log", "type": "n8n-nodes-base.set" },
                { "name": "DB", "type": "n8n-nodes-base.postgres" }
            ],
            // Hook only feeds Log; DB is unconnected → no source→sink path.
            "connections": { "Hook": { "main": [[{ "node": "Log" }]] } }
        });
        let out = analyze(&wf);
        assert!(
            out.paths.is_empty(),
            "no exposure ⇒ no path, not a fake one"
        );
    }

    #[test]
    fn unknown_node_types_are_reported_not_dropped() {
        let wf = json!({
            "nodes": [
                { "name": "Hook", "type": "n8n-nodes-base.webhook" },
                { "name": "Weird", "type": "n8n-nodes-base.someVendorThing" },
                { "name": "Send", "type": "n8n-nodes-base.httpRequest" }
            ],
            // Unknown node is a pass-through; the path is still found.
            "connections": {
                "Hook": { "main": [[{ "node": "Weird" }]] },
                "Weird": { "main": [[{ "node": "Send" }]] }
            }
        });
        let out = analyze(&wf);
        assert_eq!(out.paths.len(), 1, "unknown node must not hide the path");
        assert_eq!(
            out.unmapped_node_types,
            vec!["n8n-nodes-base.someVendorThing"]
        );
    }

    #[test]
    fn garbage_definition_yields_no_paths() {
        assert!(analyze(&json!({})).paths.is_empty());
        assert!(analyze(&json!({ "nodes": "not-an-array" }))
            .paths
            .is_empty());
        assert!(analyze(&json!(42)).paths.is_empty());
    }
}
