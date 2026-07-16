//! Bounded, deterministic Bash syntax analysis for proposed shell actions.
//!
//! The analyzer emits neutral facts only. It never executes commands, reads
//! the filesystem or environment, or chooses an authorization effect.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Component, Path, PathBuf};

use tl_core::ShellActionParameters;
use tree_sitter::{Node, Parser};

const MAX_COMMAND_BYTES: usize = 65_536;
const MAX_NODES: usize = 20_000;
const MAX_INVOCATIONS: usize = 1_024;
const MAX_RECURSION: usize = 4;
const MAX_FACTS_PER_KEY: usize = 1_024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShellAnalysisStatus {
    Complete,
    Partial,
    Unavailable,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShellAnalysis {
    pub status: ShellAnalysisStatus,
    pub facts: BTreeMap<String, BTreeSet<String>>,
    pub diagnostics: Vec<String>,
}

impl ShellAnalysis {
    pub fn has_fact(&self, key: &str, value: &str) -> bool {
        self.facts
            .get(key)
            .is_some_and(|values| values.contains(value))
    }
}

#[derive(Debug, Default)]
struct AnalysisState {
    facts: BTreeMap<String, BTreeSet<String>>,
    diagnostics: BTreeSet<String>,
    nodes: usize,
    invocations: usize,
    partial: bool,
}

impl AnalysisState {
    fn fact(&mut self, key: &str, value: &str) {
        let values = self.facts.entry(key.to_string()).or_default();
        if values.len() < MAX_FACTS_PER_KEY {
            values.insert(value.to_string());
        } else {
            self.mark_partial("fact_limit_exceeded");
        }
    }

    fn mark_partial(&mut self, diagnostic: &str) {
        self.partial = true;
        self.diagnostics.insert(diagnostic.to_string());
    }
}

pub fn analyze_shell_command(parameters: &ShellActionParameters) -> ShellAnalysis {
    if parameters.command.len() > MAX_COMMAND_BYTES {
        return ShellAnalysis {
            status: ShellAnalysisStatus::Unavailable,
            facts: BTreeMap::new(),
            diagnostics: vec!["command_size_limit_exceeded".into()],
        };
    }

    let mut state = AnalysisState::default();
    analyze_source(&parameters.command, parameters, 0, &mut state);
    let status = if state.partial {
        ShellAnalysisStatus::Partial
    } else {
        ShellAnalysisStatus::Complete
    };
    ShellAnalysis {
        status,
        facts: state.facts,
        diagnostics: state.diagnostics.into_iter().collect(),
    }
}

fn analyze_source(
    source: &str,
    parameters: &ShellActionParameters,
    depth: usize,
    state: &mut AnalysisState,
) {
    if depth > MAX_RECURSION {
        state.mark_partial("shell_recursion_limit_exceeded");
        return;
    }
    let mut parser = Parser::new();
    let language = tree_sitter_bash::LANGUAGE.into();
    if parser.set_language(&language).is_err() {
        state.mark_partial("shell_parser_unavailable");
        return;
    }
    let Some(tree) = parser.parse(source, None) else {
        state.mark_partial("shell_parse_unavailable");
        return;
    };
    if tree.root_node().has_error() {
        state.mark_partial("shell_parse_error");
    }
    walk(
        tree.root_node(),
        source.as_bytes(),
        parameters,
        depth,
        state,
    );
}

fn walk(
    node: Node<'_>,
    source: &[u8],
    parameters: &ShellActionParameters,
    depth: usize,
    state: &mut AnalysisState,
) {
    state.nodes = state.nodes.saturating_add(1);
    if state.nodes > MAX_NODES {
        state.mark_partial("shell_node_limit_exceeded");
        return;
    }

    if node.kind() == "pipeline" {
        state.fact("shell.pipeline", "true");
        classify_pipeline(node, source, state);
    }
    if node.kind() == "command" {
        state.invocations = state.invocations.saturating_add(1);
        if state.invocations > MAX_INVOCATIONS {
            state.mark_partial("shell_invocation_limit_exceeded");
            return;
        }
        analyze_command(node, source, parameters, depth, state);
    }
    if node.kind().contains("redirect") {
        analyze_redirection(node, source, parameters, state);
    }

    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        walk(child, source, parameters, depth, state);
        if state.nodes > MAX_NODES || state.invocations > MAX_INVOCATIONS {
            break;
        }
    }
}

fn analyze_command(
    node: Node<'_>,
    source: &[u8],
    parameters: &ShellActionParameters,
    depth: usize,
    state: &mut AnalysisState,
) {
    let Some((mut program, mut arguments, dynamic)) = command_words(node, source) else {
        state.fact("shell.dynamic", "true");
        state.mark_partial("dynamic_shell_value");
        return;
    };
    if dynamic {
        state.fact("shell.dynamic", "true");
        state.mark_partial("dynamic_shell_value");
    }

    normalize_wrappers(&mut program, &mut arguments, state);
    let basename = Path::new(&program)
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or(&program)
        .to_ascii_lowercase();
    state.fact("shell.program", &basename);

    if matches!(basename.as_str(), "bash" | "sh" | "zsh") {
        if let Some(index) = arguments.iter().position(|argument| argument == "-c") {
            state.fact("shell.wrapper", "shell_c");
            match arguments.get(index + 1) {
                Some(payload) if !payload.is_empty() => {
                    analyze_source(payload, parameters, depth + 1, state);
                }
                _ => {
                    state.fact("shell.dynamic", "true");
                    state.fact("shell.risk", "dynamic_evaluation");
                    state.mark_partial("dynamic_shell_payload");
                }
            }
        }
    }

    classify_program(&basename, &arguments, parameters, state);
}

fn command_words(node: Node<'_>, source: &[u8]) -> Option<(String, Vec<String>, bool)> {
    let name_node = node.child_by_field_name("name")?;
    let (program, mut dynamic) = static_word(name_node, source);
    let program = program?;
    let mut arguments = Vec::new();
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        if child.id() == name_node.id()
            || child.kind().contains("redirect")
            || child.kind() == "variable_assignment"
        {
            continue;
        }
        if is_word_node(child.kind()) {
            let (value, child_dynamic) = static_word(child, source);
            dynamic |= child_dynamic;
            if let Some(value) = value {
                arguments.push(value);
            }
        } else if child.kind() != "comment" {
            // Expansions and substitutions are named Bash nodes rather than
            // static word nodes. Retain that uncertainty even though there is
            // deliberately no attempt to resolve their runtime value.
            dynamic = true;
        }
    }
    Some((program, arguments, dynamic))
}

fn is_word_node(kind: &str) -> bool {
    matches!(
        kind,
        "word" | "raw_string" | "string" | "concatenation" | "ansi_c_string"
    )
}

fn static_word(node: Node<'_>, source: &[u8]) -> (Option<String>, bool) {
    let Ok(text) = node.utf8_text(source) else {
        return (None, true);
    };
    let text = text.trim();
    let dynamic = text.contains('$')
        || text.contains('`')
        || text.contains("$(")
        || (node.kind() != "raw_string"
            && (text.contains('*') || text.contains('?') || text.contains('[')));
    if dynamic {
        return (None, true);
    }
    let unquoted = if text.len() >= 2
        && ((text.starts_with('\'') && text.ends_with('\''))
            || (text.starts_with('"') && text.ends_with('"')))
    {
        &text[1..text.len() - 1]
    } else {
        text
    };
    (
        Some(unquoted.replace("\\ ", " ").replace("\\\"", "\"")),
        false,
    )
}

fn normalize_wrappers(
    program: &mut String,
    arguments: &mut Vec<String>,
    state: &mut AnalysisState,
) {
    loop {
        let base = Path::new(program.as_str())
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or(program.as_str())
            .to_ascii_lowercase();
        if !matches!(
            base.as_str(),
            "sudo" | "env" | "command" | "builtin" | "nohup" | "time"
        ) {
            break;
        }
        state.fact("shell.wrapper", &base);
        let next = arguments.iter().position(|argument| {
            !(argument.starts_with('-') || base == "env" && argument.contains('='))
        });
        let Some(index) = next else {
            state.fact("shell.dynamic", "true");
            state.mark_partial("dynamic_shell_value");
            return;
        };
        *program = arguments[index].clone();
        *arguments = arguments.split_off(index + 1);
    }
}

fn classify_program(
    program: &str,
    arguments: &[String],
    parameters: &ShellActionParameters,
    state: &mut AnalysisState,
) {
    match program {
        "rm" => classify_rm(arguments, parameters, state),
        "find" => classify_find(arguments, parameters, state),
        "truncate" => {
            state.fact("shell.risk", "filesystem_overwrite");
            classify_targets(arguments, parameters, state);
        }
        "git" => classify_git(arguments, state),
        "dd" => classify_dd(arguments, parameters, state),
        "mkfs" | "mkfs.ext2" | "mkfs.ext3" | "mkfs.ext4" | "wipefs" => {
            state.fact("shell.risk", "disk_overwrite");
            classify_targets(arguments, parameters, state);
        }
        "docker" | "podman"
            if arguments
                .iter()
                .any(|value| matches!(value.as_str(), "rm" | "prune")) =>
        {
            state.fact("shell.risk", "container_destructive");
        }
        "kubectl" if arguments.first().is_some_and(|value| value == "delete") => {
            state.fact("shell.risk", "infrastructure_destroy");
        }
        "helm"
            if arguments
                .first()
                .is_some_and(|value| matches!(value.as_str(), "uninstall" | "delete")) =>
        {
            state.fact("shell.risk", "infrastructure_destroy");
        }
        "terraform" | "tofu" if arguments.first().is_some_and(|value| value == "destroy") => {
            state.fact("shell.risk", "infrastructure_destroy");
        }
        "chmod" | "chown" if arguments.iter().any(|value| flag_contains(value, 'R')) => {
            state.fact("shell.risk", "privilege_change");
            classify_targets(arguments, parameters, state);
        }
        "kill" | "killall" | "pkill" => state.fact("shell.risk", "process_termination"),
        "eval" => {
            state.fact("shell.dynamic", "true");
            state.fact("shell.risk", "dynamic_evaluation");
            state.mark_partial("dynamic_shell_value");
        }
        "psql" | "mysql" | "sqlite3" | "redis-cli" => {
            let lowered = arguments.join(" ").to_ascii_lowercase();
            if [
                "drop database",
                "drop table",
                "drop schema",
                "truncate ",
                "flushall",
                "flushdb",
            ]
            .iter()
            .any(|needle| lowered.contains(needle))
            {
                state.fact("shell.risk", "database_destructive");
            }
        }
        "xargs" => {
            state.fact("shell.wrapper", "xargs");
            if arguments.iter().any(|value| value == "rm") {
                state.fact("shell.risk", "filesystem_recursive_delete");
            }
        }
        _ => {}
    }
}

fn classify_rm(
    arguments: &[String],
    parameters: &ShellActionParameters,
    state: &mut AnalysisState,
) {
    let recursive = arguments.iter().any(|value| {
        value == "--recursive" || flag_contains(value, 'r') || flag_contains(value, 'R')
    });
    let force = arguments
        .iter()
        .any(|value| value == "--force" || flag_contains(value, 'f'));
    if recursive {
        state.fact("shell.flag", "recursive");
        state.fact("shell.risk", "filesystem_recursive_delete");
    }
    if force {
        state.fact("shell.flag", "force");
    }
    if arguments.iter().any(|value| value == "--no-preserve-root") {
        state.fact("shell.flag", "no_preserve_root");
    }
    classify_targets(arguments, parameters, state);
}

fn classify_find(
    arguments: &[String],
    parameters: &ShellActionParameters,
    state: &mut AnalysisState,
) {
    if arguments
        .iter()
        .any(|value| matches!(value.as_str(), "-delete" | "-exec" | "-execdir"))
    {
        state.fact("shell.risk", "filesystem_recursive_delete");
        classify_targets(arguments, parameters, state);
    }
}

fn classify_git(arguments: &[String], state: &mut AnalysisState) {
    let first = arguments.first().map(String::as_str);
    if first == Some("reset") && arguments.iter().any(|value| value == "--hard") {
        state.fact("shell.flag", "hard");
        state.fact("shell.risk", "vcs_history_rewrite");
        state.fact("shell.target_scope", "vcs_metadata");
    }
    if first == Some("clean")
        && arguments.iter().any(|value| flag_contains(value, 'f'))
        && arguments
            .iter()
            .any(|value| flag_contains(value, 'd') || flag_contains(value, 'x'))
    {
        state.fact("shell.risk", "vcs_untracked_delete");
        state.fact("shell.target_scope", "workspace");
    }
    if first == Some("push")
        && arguments.iter().any(|value| {
            value == "--force" || value == "--force-with-lease" || flag_contains(value, 'f')
        })
    {
        state.fact("shell.risk", "vcs_history_rewrite");
        state.fact("shell.target_scope", "vcs_metadata");
    }
}

fn classify_dd(
    arguments: &[String],
    parameters: &ShellActionParameters,
    state: &mut AnalysisState,
) {
    if let Some(target) = arguments.iter().find_map(|value| value.strip_prefix("of=")) {
        state.fact("shell.risk", "filesystem_overwrite");
        if target.starts_with("/dev/") {
            state.fact("shell.risk", "disk_overwrite");
        }
        classify_target(target, parameters, state);
    }
}

fn classify_targets(
    arguments: &[String],
    parameters: &ShellActionParameters,
    state: &mut AnalysisState,
) {
    let targets = arguments.iter().filter(|value| {
        !value.starts_with('-')
            && !value.contains('=')
            && !matches!(value.as_str(), "{}" | ";" | "+")
    });
    let mut found = false;
    for target in targets {
        found = true;
        classify_target(target, parameters, state);
    }
    if !found {
        state.fact("shell.target_scope", "unknown");
    }
}

fn classify_target(target: &str, parameters: &ShellActionParameters, state: &mut AnalysisState) {
    if target.contains('$') || target.contains('*') || target.contains('?') {
        state.fact("shell.dynamic", "true");
        state.fact("shell.target_scope", "unknown");
        state.mark_partial("dynamic_shell_value");
        return;
    }
    if target == "/" || target == "//" {
        state.fact("shell.target_scope", "root");
        return;
    }
    if target == ".git" || target.starts_with(".git/") || target.contains("/.git/") {
        state.fact("shell.target_scope", "vcs_metadata");
        return;
    }
    if target == "~" || target.starts_with("~/") {
        state.fact("shell.target_scope", "home");
        return;
    }
    if target.starts_with("/tmp/") || target == "/tmp" || target.starts_with("/var/tmp/") {
        state.fact("shell.target_scope", "temporary");
        return;
    }
    if [
        "/etc", "/usr", "/bin", "/sbin", "/var", "/opt", "/boot", "/dev", "/System", "/Library",
    ]
    .iter()
    .any(|prefix| target == *prefix || target.starts_with(&format!("{prefix}/")))
    {
        state.fact("shell.target_scope", "system");
        return;
    }

    let path = lexical_path(target, parameters.cwd.as_deref());
    if let (Some(path), Some(root)) = (path.as_ref(), parameters.workspace_root.as_deref()) {
        let root = normalize_path(Path::new(root));
        if path.starts_with(root) {
            state.fact("shell.target_scope", "workspace");
        } else {
            state.fact("shell.target_scope", "outside_workspace");
        }
    } else if Path::new(target).is_absolute() {
        state.fact("shell.target_scope", "outside_workspace");
    } else {
        state.fact("shell.target_scope", "workspace");
    }
}

fn lexical_path(target: &str, cwd: Option<&str>) -> Option<PathBuf> {
    let target = Path::new(target);
    if target.is_absolute() {
        Some(normalize_path(target))
    } else {
        cwd.map(|cwd| normalize_path(&Path::new(cwd).join(target)))
    }
}

fn normalize_path(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            other => normalized.push(other.as_os_str()),
        }
    }
    normalized
}

fn analyze_redirection(
    node: Node<'_>,
    source: &[u8],
    parameters: &ShellActionParameters,
    state: &mut AnalysisState,
) {
    let Ok(text) = node.utf8_text(source) else {
        state.mark_partial("redirect_decode_error");
        return;
    };
    if text.contains(">>") {
        state.fact("shell.redirection", "append");
        return;
    }
    if let Some((_, target)) = text.rsplit_once('>') {
        state.fact("shell.redirection", "overwrite");
        state.fact("shell.risk", "filesystem_overwrite");
        classify_target(target.trim(), parameters, state);
    }
}

fn classify_pipeline(node: Node<'_>, source: &[u8], state: &mut AnalysisState) {
    let mut programs = BTreeSet::new();
    collect_programs(node, source, &mut programs);
    let downloads = programs.contains("curl") || programs.contains("wget");
    let shells = programs.contains("sh") || programs.contains("bash") || programs.contains("zsh");
    if downloads && shells {
        state.fact("shell.risk", "download_execute");
    }
}

fn collect_programs(node: Node<'_>, source: &[u8], programs: &mut BTreeSet<String>) {
    if node.kind() == "command" {
        if let Some(name) = node.child_by_field_name("name") {
            if let (Some(value), false) = static_word(name, source) {
                let base = Path::new(&value)
                    .file_name()
                    .and_then(|item| item.to_str())
                    .unwrap_or(&value)
                    .to_ascii_lowercase();
                programs.insert(base);
            }
        }
    }
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        collect_programs(child, source, programs);
    }
}

fn flag_contains(argument: &str, flag: char) -> bool {
    argument.starts_with('-')
        && !argument.starts_with("--")
        && argument.chars().skip(1).any(|value| value == flag)
}
