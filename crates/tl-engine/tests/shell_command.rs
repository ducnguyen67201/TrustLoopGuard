use std::collections::BTreeMap;

use serde::Deserialize;
use tl_core::{ShellActionParameters, ShellLanguage};
use tl_engine::{analyze_shell_command, ShellAnalysisStatus};

#[derive(Debug, Deserialize)]
struct Case {
    name: String,
    command: String,
    status: String,
    #[serde(default)]
    facts: BTreeMap<String, Vec<String>>,
    #[serde(default)]
    absent: BTreeMap<String, Vec<String>>,
}

#[test]
fn shell_command_corpus_emits_only_bounded_neutral_facts() {
    let cases: Vec<Case> = serde_yaml::from_str(include_str!("fixtures/shell-command-cases.yaml"))
        .expect("fixture parses");

    for case in cases {
        let result = analyze_shell_command(&ShellActionParameters {
            command: case.command,
            shell: ShellLanguage::Bash,
            cwd: Some("/workspace/project".into()),
            workspace_root: Some("/workspace/project".into()),
            timeout_ms: None,
            run_in_background: false,
        });
        let expected_status = match case.status.as_str() {
            "complete" => ShellAnalysisStatus::Complete,
            "partial" => ShellAnalysisStatus::Partial,
            other => panic!("unknown fixture status {other}"),
        };
        assert_eq!(result.status, expected_status, "{}", case.name);

        for (key, values) in case.facts {
            for value in values {
                assert!(
                    result.has_fact(&key, &value),
                    "{}: missing {key}={value}",
                    case.name
                );
            }
        }
        for (key, values) in case.absent {
            for value in values {
                assert!(
                    !result.has_fact(&key, &value),
                    "{}: unexpected {key}={value}",
                    case.name
                );
            }
        }
    }
}

#[test]
fn oversized_commands_are_unavailable_without_parsing() {
    let result = analyze_shell_command(&ShellActionParameters {
        command: "x".repeat(65_537),
        shell: ShellLanguage::Bash,
        cwd: None,
        workspace_root: None,
        timeout_ms: None,
        run_in_background: false,
    });

    assert_eq!(result.status, ShellAnalysisStatus::Unavailable);
    assert!(result.facts.is_empty());
}
