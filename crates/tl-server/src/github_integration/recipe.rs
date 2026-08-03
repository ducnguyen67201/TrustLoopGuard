use serde::Deserialize;
use serde_json::json;
use tl_core::{
    GitHubConnectionSummary, GitHubIntegrationAnalysisSummary, GitHubIntegrationManualStep,
    GitHubProposedFileChange, GitHubProposedFileOperation,
    GITHUB_INTEGRATION_RECIPE_TYPESCRIPT_NEXTJS_V1,
};
use tl_llm::{JsonSchema, LlmRouteKind, LlmRouter};

use super::github_client::{GitHubClient, GitHubFile};
use super::validation::{
    contains_required_marker, is_probably_binary, sha256_hex, validate_candidate_path,
    MAX_CONTEXT_FILES, MAX_FILE_BYTES, MAX_GENERATED_BYTES, MAX_PROPOSED_FILES,
    MAX_TOTAL_CONTEXT_BYTES,
};
use super::{GitHubClientError, GitHubIntegrationStoreError};

#[derive(Debug, Clone)]
pub struct AnalysisResult {
    pub base_sha: String,
    pub summary: GitHubIntegrationAnalysisSummary,
    pub proposed_changes: Vec<GitHubProposedFileChange>,
    pub manual_steps: Vec<GitHubIntegrationManualStep>,
}

pub async fn analyze(
    github: &dyn GitHubClient,
    llm: &LlmRouter,
    installation_id: i64,
    connection: &GitHubConnectionSummary,
    risk_statement: &str,
) -> Result<AnalysisResult, GitHubIntegrationStoreError> {
    let (base_sha, tree, truncated) = github
        .get_tree(
            installation_id,
            &connection.owner,
            &connection.name,
            &connection.default_branch,
        )
        .await
        .map_err(github_error)?;
    if truncated {
        return Err(GitHubIntegrationStoreError::Validation(
            "GitHub tree was truncated; choose a narrower repository root".into(),
        ));
    }
    let candidates = ranked_candidates(&connection.root_path, tree)?;
    if candidates.is_empty() {
        return Err(GitHubIntegrationStoreError::Validation(
            "no supported TypeScript/Next.js integration point found".into(),
        ));
    }

    let mut files = Vec::new();
    let mut total_bytes = 0usize;
    for path in candidates.into_iter().take(MAX_CONTEXT_FILES) {
        let file = github
            .get_file(
                installation_id,
                &connection.owner,
                &connection.name,
                &path,
                &base_sha,
            )
            .await
            .map_err(github_error)?;
        if file.bytes.len() > MAX_FILE_BYTES || is_probably_binary(&file.bytes) {
            continue;
        }
        total_bytes += file.bytes.len();
        if total_bytes > MAX_TOTAL_CONTEXT_BYTES {
            break;
        }
        files.push(file);
    }
    if files.is_empty() {
        return Err(GitHubIntegrationStoreError::Validation(
            "candidate files were empty, binary, or too large".into(),
        ));
    }

    let prompt = prompt(connection, risk_statement, &files);
    let out = llm
        .complete_route(LlmRouteKind::GitHubIntegration, &prompt, &proposal_schema())
        .await
        .map_err(|e| {
            GitHubIntegrationStoreError::Unavailable(format!("llm provider error: {e}"))
        })?;
    let response: ProposalResponse = serde_json::from_value(out.json).map_err(|e| {
        GitHubIntegrationStoreError::Unavailable(format!("llm response parse: {e}"))
    })?;
    let mut proposed_changes = Vec::with_capacity(response.file_replacements.len());
    let mut total_generated = 0usize;
    for (idx, change) in response.file_replacements.into_iter().enumerate() {
        if idx >= MAX_PROPOSED_FILES {
            return Err(GitHubIntegrationStoreError::Validation(
                "proposal edits too many files".into(),
            ));
        }
        let path = validate_candidate_path(&change.path)
            .map_err(GitHubIntegrationStoreError::Validation)?;
        if change.replacement.len() > MAX_GENERATED_BYTES {
            return Err(GitHubIntegrationStoreError::Validation(
                "proposal generated content exceeds size limit".into(),
            ));
        }
        total_generated += change.replacement.len();
        if total_generated > MAX_GENERATED_BYTES {
            return Err(GitHubIntegrationStoreError::Validation(
                "proposal generated content exceeds total size limit".into(),
            ));
        }
        if !contains_required_marker(&change.replacement, &connection.id) {
            return Err(GitHubIntegrationStoreError::Validation(
                "proposal is missing the Featherlane AI integration marker".into(),
            ));
        }
        if change.rationale.trim().is_empty() {
            return Err(GitHubIntegrationStoreError::Validation(
                "every proposed file needs a rationale".into(),
            ));
        }
        proposed_changes.push(GitHubProposedFileChange {
            path,
            operation: change.operation,
            content_sha: change.content_sha,
            replacement: change.replacement,
            rationale: change.rationale,
        });
    }
    if proposed_changes.is_empty() {
        return Err(GitHubIntegrationStoreError::Validation(
            "proposal did not include any file changes".into(),
        ));
    }
    Ok(AnalysisResult {
        base_sha,
        summary: GitHubIntegrationAnalysisSummary {
            detected_framework: response.detected_framework,
            package_manager: response.package_manager,
            summary: response.summary,
            integration_points: response.integration_points,
        },
        proposed_changes,
        manual_steps: response.manual_steps,
    })
}

fn ranked_candidates(
    root_path: &str,
    tree: Vec<super::GitHubTreeEntry>,
) -> Result<Vec<String>, GitHubIntegrationStoreError> {
    let root = root_path.trim_matches('/');
    let mut scored = Vec::new();
    for entry in tree {
        if entry.kind != "blob" {
            continue;
        }
        if entry.size.unwrap_or(0) > MAX_FILE_BYTES as i64 {
            continue;
        }
        let path = entry.path;
        if !root.is_empty() && !path.starts_with(&format!("{root}/")) {
            continue;
        }
        let path =
            validate_candidate_path(&path).map_err(GitHubIntegrationStoreError::Validation)?;
        let lowered = path.to_ascii_lowercase();
        if !(lowered.ends_with(".ts")
            || lowered.ends_with(".tsx")
            || lowered.ends_with(".js")
            || lowered.ends_with(".jsx")
            || lowered.ends_with("package.json")
            || lowered.ends_with("tsconfig.json"))
        {
            continue;
        }
        if lowered.contains(".test.")
            || lowered.contains(".spec.")
            || lowered.contains("__snapshots__")
            || lowered.contains(".generated.")
        {
            continue;
        }
        let score = if lowered.ends_with("package.json") {
            100
        } else if lowered.contains("/app/api/") || lowered.contains("/pages/api/") {
            90
        } else if lowered.contains("agent")
            || lowered.contains("openai")
            || lowered.contains("anthropic")
        {
            75
        } else if lowered.ends_with("next.config.ts") || lowered.ends_with("next.config.js") {
            60
        } else {
            10
        };
        scored.push((score, path));
    }
    scored.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(&b.1)));
    Ok(scored.into_iter().map(|(_, path)| path).collect())
}

fn prompt(
    connection: &GitHubConnectionSummary,
    risk_statement: &str,
    files: &[GitHubFile],
) -> String {
    let mut prompt = format!(
        "You are preparing a Featherlane AI SDK integration for a TypeScript/Next.js repository.\n\
         Return only JSON matching the schema.\n\
         Recipe: {GITHUB_INTEGRATION_RECIPE_TYPESCRIPT_NEXTJS_V1}\n\
         Agent ID: {}\n\
         Environment ID: {}\n\
         Integration marker: {}\n\
         Risk statement: {}\n\
         Required context object in every guard call: {{ featherlane_ai_integration_id: \"{}\", featherlane_ai_recipe_version: \"{}\" }}\n\
         The API key must be referenced as process.env.FEATHERLANE_AI_API_KEY. Never hardcode a key.\n\
         If @featherlane-ai/sdk is absent, update only package.json and include a manual lockfile refresh step.\n\
         Do not edit workflows, env files, lockfiles, generated files, or unrelated code.\n\n",
        connection.agent_id,
        connection.environment_id,
        connection.id,
        risk_statement,
        connection.id,
        GITHUB_INTEGRATION_RECIPE_TYPESCRIPT_NEXTJS_V1,
    );
    for file in files {
        let text = String::from_utf8_lossy(&file.bytes);
        prompt.push_str(&format!(
            "\n--- FILE path={} sha={} content_sha={} ---\n{}\n",
            file.path,
            file.sha,
            sha256_hex(&file.bytes),
            text
        ));
    }
    prompt
}

#[derive(Debug, Deserialize)]
struct ProposalResponse {
    detected_framework: String,
    package_manager: String,
    summary: String,
    integration_points: Vec<String>,
    file_replacements: Vec<ProposalFileReplacement>,
    manual_steps: Vec<GitHubIntegrationManualStep>,
}

#[derive(Debug, Deserialize)]
struct ProposalFileReplacement {
    path: String,
    operation: GitHubProposedFileOperation,
    content_sha: String,
    replacement: String,
    rationale: String,
}

fn proposal_schema() -> JsonSchema {
    JsonSchema {
        name: "featherlane_ai_github_integration_plan".into(),
        schema: json!({
            "type": "object",
            "additionalProperties": false,
            "required": [
                "detected_framework",
                "package_manager",
                "summary",
                "integration_points",
                "file_replacements",
                "manual_steps"
            ],
            "properties": {
                "detected_framework": { "type": "string" },
                "package_manager": { "type": "string" },
                "summary": { "type": "string" },
                "integration_points": {
                    "type": "array",
                    "items": { "type": "string" },
                    "maxItems": 10
                },
                "file_replacements": {
                    "type": "array",
                    "minItems": 1,
                    "maxItems": MAX_PROPOSED_FILES,
                    "items": {
                        "type": "object",
                        "additionalProperties": false,
                        "required": ["path", "operation", "content_sha", "replacement", "rationale"],
                        "properties": {
                            "path": { "type": "string" },
                            "operation": { "type": "string", "enum": ["create", "update"] },
                            "content_sha": { "type": "string" },
                            "replacement": { "type": "string" },
                            "rationale": { "type": "string" }
                        }
                    }
                },
                "manual_steps": {
                    "type": "array",
                    "maxItems": 8,
                    "items": {
                        "type": "object",
                        "additionalProperties": false,
                        "required": ["label", "command", "reason"],
                        "properties": {
                            "label": { "type": "string" },
                            "command": { "type": "string" },
                            "reason": { "type": "string" }
                        }
                    }
                }
            }
        }),
    }
}

fn github_error(error: GitHubClientError) -> GitHubIntegrationStoreError {
    match error {
        GitHubClientError::NotFound => GitHubIntegrationStoreError::NotFound,
        GitHubClientError::Conflict => GitHubIntegrationStoreError::Conflict,
        GitHubClientError::Auth => {
            GitHubIntegrationStoreError::Unavailable("GitHub authorization failed".into())
        }
        other => GitHubIntegrationStoreError::Unavailable(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    use async_trait::async_trait;
    use tl_core::{GitHubConnectionStatus, GITHUB_INTEGRATION_RECIPE_TYPESCRIPT_NEXTJS_V1};
    use tl_llm::{LlmClient, LlmError, LlmOutput, ProviderTarget, ResolvedRoute, TokenBudget};

    use super::*;
    use crate::github_integration::github_client::GitHubInstallationProof;
    use crate::github_integration::{
        GitHubDraftPrRequest, GitHubPullRequest, GitHubRepository, GitHubTreeEntry,
    };

    struct FixtureGitHub;

    #[async_trait]
    impl GitHubClient for FixtureGitHub {
        async fn verify_callback_installation(
            &self,
            _code: &str,
            _installation_id: i64,
        ) -> Result<GitHubInstallationProof, GitHubClientError> {
            unreachable!("analysis does not verify installations")
        }

        async fn list_repositories(
            &self,
            _installation_id: i64,
        ) -> Result<Vec<GitHubRepository>, GitHubClientError> {
            unreachable!("analysis does not list repositories")
        }

        async fn get_tree(
            &self,
            _installation_id: i64,
            _owner: &str,
            _repo: &str,
            _branch: &str,
        ) -> Result<(String, Vec<GitHubTreeEntry>, bool), GitHubClientError> {
            Ok((
                "base-sha".into(),
                vec![GitHubTreeEntry {
                    path: "app/api/agent.ts".into(),
                    sha: "file-sha".into(),
                    kind: "blob".into(),
                    size: Some(32),
                }],
                false,
            ))
        }

        async fn get_file(
            &self,
            _installation_id: i64,
            _owner: &str,
            _repo: &str,
            path: &str,
            _reference: &str,
        ) -> Result<GitHubFile, GitHubClientError> {
            Ok(GitHubFile {
                path: path.into(),
                sha: "file-sha".into(),
                bytes: b"export async function agent() {}".to_vec(),
            })
        }

        async fn create_draft_pr(
            &self,
            _request: GitHubDraftPrRequest,
        ) -> Result<GitHubPullRequest, GitHubClientError> {
            unreachable!("analysis does not create a pull request")
        }
    }

    struct RecordingLlm {
        calls: Arc<Mutex<Vec<(String, Duration)>>>,
        fail: bool,
    }

    #[async_trait]
    impl LlmClient for RecordingLlm {
        async fn complete(
            &self,
            model: &str,
            _prompt: &str,
            _schema: &JsonSchema,
            deadline: Duration,
        ) -> Result<LlmOutput, LlmError> {
            self.calls
                .lock()
                .expect("calls lock poisoned")
                .push((model.into(), deadline));
            if self.fail {
                return Err(LlmError::Http("fixture provider failed".into()));
            }
            Ok(LlmOutput {
                json: json!({
                    "detected_framework": "Next.js",
                    "package_manager": "pnpm",
                    "summary": "Add Featherlane AI guard calls",
                    "integration_points": ["app/api/agent.ts"],
                    "file_replacements": [{
                        "path": "app/api/agent.ts",
                        "operation": "update",
                        "content_sha": "fixture-content-sha",
                        "replacement": "const featherlane_ai_integration_id = \"connection-1\";\nconst key = process.env.FEATHERLANE_AI_API_KEY;",
                        "rationale": "Guard the agent boundary"
                    }],
                    "manual_steps": [{
                        "label": "Install SDK",
                        "command": "pnpm install",
                        "reason": "Refresh dependencies"
                    }]
                }),
                prompt_tokens: 10,
                completion_tokens: 5,
            })
        }
    }

    fn connection() -> GitHubConnectionSummary {
        GitHubConnectionSummary {
            id: "connection-1".into(),
            workspace_id: "workspace-1".into(),
            installation_id: "installation-1".into(),
            repository_id: "repository-1".into(),
            owner: "acme".into(),
            name: "agent-app".into(),
            default_branch: "main".into(),
            root_path: String::new(),
            agent_id: "agent-1".into(),
            environment_id: "production".into(),
            status: GitHubConnectionStatus::Active,
            recipe_version: GITHUB_INTEGRATION_RECIPE_TYPESCRIPT_NEXTJS_V1.into(),
            created_at: "2026-08-03T00:00:00Z".into(),
            updated_at: "2026-08-03T00:00:00Z".into(),
        }
    }

    fn router(calls: Arc<Mutex<Vec<(String, Duration)>>>, fail: bool) -> LlmRouter {
        let mut providers: HashMap<String, Arc<dyn LlmClient>> = HashMap::new();
        providers.insert("openai".into(), Arc::new(RecordingLlm { calls, fail }));
        let mut routes = HashMap::new();
        routes.insert(
            LlmRouteKind::GitHubIntegration,
            ResolvedRoute {
                primary: ProviderTarget {
                    provider: "openai".into(),
                    model: "github-route-model".into(),
                    deadline_ms: 60_000,
                    reasoning_effort: None,
                },
                fallback: None,
            },
        );
        LlmRouter::new(providers, routes, Arc::new(TokenBudget::new(0)))
    }

    #[tokio::test]
    async fn analysis_uses_the_github_route_model_and_deadline() {
        let calls = Arc::new(Mutex::new(Vec::new()));
        let result = analyze(
            &FixtureGitHub,
            &router(calls.clone(), false),
            7,
            &connection(),
            "Prevent unguarded high-stakes actions",
        )
        .await
        .expect("analysis");

        assert_eq!(result.base_sha, "base-sha");
        assert_eq!(result.proposed_changes.len(), 1);
        assert_eq!(
            *calls.lock().expect("calls lock poisoned"),
            vec![("github-route-model".into(), Duration::from_secs(60))]
        );
    }

    #[tokio::test]
    async fn provider_failure_remains_unavailable() {
        let error = analyze(
            &FixtureGitHub,
            &router(Arc::new(Mutex::new(Vec::new())), true),
            7,
            &connection(),
            "Prevent unguarded high-stakes actions",
        )
        .await
        .expect_err("provider should fail");

        assert!(matches!(error, GitHubIntegrationStoreError::Unavailable(_)));
    }
}
