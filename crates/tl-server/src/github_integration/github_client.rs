use std::sync::Mutex;

use async_trait::async_trait;
use chrono::{Duration as ChronoDuration, Utc};
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use reqwest::header::{ACCEPT, AUTHORIZATION, USER_AGENT};
use serde::{Deserialize, Serialize};

use super::config::{GitHubAppConfig, GITHUB_REQUEST_TIMEOUT};

const API_VERSION_HEADER: &str = "X-GitHub-Api-Version";
const API_VERSION: &str = "2026-03-10";
const ACCEPT_JSON: &str = "application/vnd.github+json";
const USER_AGENT_VALUE: &str = "TrustLoopGuard-GitHub-Integration";

#[derive(Debug, thiserror::Error)]
pub enum GitHubClientError {
    #[error("github auth failed")]
    Auth,
    #[error("github returned status {status}")]
    Status { status: u16 },
    #[error("github response parse failed")]
    Parse,
    #[error("github transport failed")]
    Transport,
    #[error("github resource not found")]
    NotFound,
    #[error("github conflict")]
    Conflict,
}

#[derive(Debug, Clone)]
pub struct GitHubRepository {
    pub repository_id: i64,
    pub owner: String,
    pub name: String,
    pub full_name: String,
    pub default_branch: String,
    pub private: bool,
    pub archived: bool,
}

#[derive(Debug, Clone)]
pub struct GitHubTreeEntry {
    pub path: String,
    pub sha: String,
    pub kind: String,
    pub size: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct GitHubFile {
    pub path: String,
    pub sha: String,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct GitHubPullRequest {
    pub number: i64,
    pub url: String,
    pub branch_name: String,
    pub commit_sha: String,
}

#[derive(Debug, Clone)]
pub struct GitHubDraftPrRequest {
    pub installation_id: i64,
    pub owner: String,
    pub repo: String,
    pub base_branch: String,
    pub base_sha: String,
    pub branch_name: String,
    pub changes: Vec<tl_core::GitHubProposedFileChange>,
    pub title: String,
    pub body: String,
}

#[derive(Debug, Clone)]
pub struct GitHubInstallationProof {
    pub installation_id: i64,
    pub account_login: String,
    pub account_type: String,
    pub repository_selection: tl_core::GitHubRepositorySelection,
}

#[async_trait]
pub trait GitHubClient: Send + Sync {
    async fn verify_callback_installation(
        &self,
        code: &str,
        installation_id: i64,
    ) -> Result<GitHubInstallationProof, GitHubClientError>;
    async fn list_repositories(
        &self,
        installation_id: i64,
    ) -> Result<Vec<GitHubRepository>, GitHubClientError>;
    async fn get_tree(
        &self,
        installation_id: i64,
        owner: &str,
        repo: &str,
        branch: &str,
    ) -> Result<(String, Vec<GitHubTreeEntry>, bool), GitHubClientError>;
    async fn get_file(
        &self,
        installation_id: i64,
        owner: &str,
        repo: &str,
        path: &str,
        reference: &str,
    ) -> Result<GitHubFile, GitHubClientError>;
    async fn create_draft_pr(
        &self,
        request: GitHubDraftPrRequest,
    ) -> Result<GitHubPullRequest, GitHubClientError>;
}

pub struct ReqwestGitHubClient {
    config: GitHubAppConfig,
    http: reqwest::Client,
    token_cache: Mutex<Option<CachedInstallationToken>>,
}

#[derive(Clone)]
struct CachedInstallationToken {
    installation_id: i64,
    token: String,
    expires_at: chrono::DateTime<Utc>,
}

impl std::fmt::Debug for ReqwestGitHubClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ReqwestGitHubClient")
            .finish_non_exhaustive()
    }
}

impl ReqwestGitHubClient {
    pub fn from_env() -> Result<Self, String> {
        Self::new(GitHubAppConfig::from_env()?)
    }

    pub fn new(config: GitHubAppConfig) -> Result<Self, String> {
        let http = reqwest::Client::builder()
            .timeout(GITHUB_REQUEST_TIMEOUT)
            .build()
            .map_err(|e| e.to_string())?;
        Ok(Self {
            config,
            http,
            token_cache: Mutex::new(None),
        })
    }

    async fn app_jwt(&self) -> Result<String, GitHubClientError> {
        #[derive(Serialize)]
        struct Claims {
            iat: i64,
            exp: i64,
            iss: String,
        }
        let now = Utc::now();
        let claims = Claims {
            iat: (now - ChronoDuration::seconds(60)).timestamp(),
            exp: (now + ChronoDuration::minutes(9)).timestamp(),
            iss: self.config.app_id.to_string(),
        };
        let mut header = Header::new(Algorithm::RS256);
        header.typ = Some("JWT".to_string());
        let key = EncodingKey::from_rsa_der(&self.config.private_key_der);
        encode(&header, &claims, &key).map_err(|_| GitHubClientError::Auth)
    }

    async fn installation_token(&self, installation_id: i64) -> Result<String, GitHubClientError> {
        if let Some(cached) = self
            .token_cache
            .lock()
            .expect("github token cache lock")
            .clone()
        {
            if cached.installation_id == installation_id
                && cached.expires_at > Utc::now() + ChronoDuration::minutes(5)
            {
                return Ok(cached.token);
            }
        }

        #[derive(Deserialize)]
        struct TokenResponse {
            token: String,
            expires_at: chrono::DateTime<Utc>,
        }
        let jwt = self.app_jwt().await?;
        let res = self
            .http
            .post(format!(
                "https://api.github.com/app/installations/{installation_id}/access_tokens"
            ))
            .headers(base_headers())
            .header(AUTHORIZATION, format!("Bearer {jwt}"))
            .json(&serde_json::json!({
                "permissions": {
                    "contents": "write",
                    "pull_requests": "write",
                    "metadata": "read"
                }
            }))
            .send()
            .await
            .map_err(|_| GitHubClientError::Transport)?;
        if !res.status().is_success() {
            return Err(status_error(res.status().as_u16()));
        }
        let token = res
            .json::<TokenResponse>()
            .await
            .map_err(|_| GitHubClientError::Parse)?;
        *self.token_cache.lock().expect("github token cache lock") =
            Some(CachedInstallationToken {
                installation_id,
                token: token.token.clone(),
                expires_at: token.expires_at,
            });
        Ok(token.token)
    }

    async fn authed_get<T: for<'de> Deserialize<'de>>(
        &self,
        installation_id: i64,
        url: String,
    ) -> Result<T, GitHubClientError> {
        let token = self.installation_token(installation_id).await?;
        let res = self
            .http
            .get(url)
            .headers(base_headers())
            .header(AUTHORIZATION, format!("Bearer {token}"))
            .send()
            .await
            .map_err(|_| GitHubClientError::Transport)?;
        if !res.status().is_success() {
            return Err(status_error(res.status().as_u16()));
        }
        res.json::<T>().await.map_err(|_| GitHubClientError::Parse)
    }
}

#[async_trait]
impl GitHubClient for ReqwestGitHubClient {
    async fn verify_callback_installation(
        &self,
        code: &str,
        installation_id: i64,
    ) -> Result<GitHubInstallationProof, GitHubClientError> {
        #[derive(Serialize)]
        struct ExchangeRequest<'a> {
            client_id: &'a str,
            client_secret: &'a str,
            code: &'a str,
        }
        #[derive(Deserialize)]
        struct ExchangeResponse {
            access_token: String,
        }
        #[derive(Deserialize)]
        struct InstallationList {
            installations: Vec<UserInstallation>,
        }
        #[derive(Deserialize)]
        struct UserInstallation {
            id: i64,
            account: Account,
            repository_selection: String,
        }
        #[derive(Deserialize)]
        struct Account {
            login: String,
            #[serde(rename = "type")]
            kind: String,
        }

        let exchange = self
            .http
            .post("https://github.com/login/oauth/access_token")
            .headers(base_headers())
            .json(&ExchangeRequest {
                client_id: &self.config.client_id,
                client_secret: &self.config.client_secret,
                code,
            })
            .send()
            .await
            .map_err(|_| GitHubClientError::Transport)?;
        if !exchange.status().is_success() {
            return Err(status_error(exchange.status().as_u16()));
        }
        let token = exchange
            .json::<ExchangeResponse>()
            .await
            .map_err(|_| GitHubClientError::Parse)?;
        let installations = self
            .http
            .get("https://api.github.com/user/installations")
            .headers(base_headers())
            .header(AUTHORIZATION, format!("Bearer {}", token.access_token))
            .send()
            .await
            .map_err(|_| GitHubClientError::Transport)?;
        if !installations.status().is_success() {
            return Err(status_error(installations.status().as_u16()));
        }
        let installations = installations
            .json::<InstallationList>()
            .await
            .map_err(|_| GitHubClientError::Parse)?;
        let Some(found) = installations
            .installations
            .into_iter()
            .find(|installation| installation.id == installation_id)
        else {
            return Err(GitHubClientError::Status { status: 403 });
        };
        Ok(GitHubInstallationProof {
            installation_id: found.id,
            account_login: found.account.login,
            account_type: found.account.kind,
            repository_selection: if found.repository_selection == "all" {
                tl_core::GitHubRepositorySelection::All
            } else {
                tl_core::GitHubRepositorySelection::Selected
            },
        })
    }

    async fn list_repositories(
        &self,
        installation_id: i64,
    ) -> Result<Vec<GitHubRepository>, GitHubClientError> {
        #[derive(Deserialize)]
        struct RepoList {
            repositories: Vec<Repo>,
        }
        #[derive(Deserialize)]
        struct Repo {
            id: i64,
            name: String,
            full_name: String,
            private: bool,
            archived: bool,
            default_branch: String,
            owner: Owner,
        }
        #[derive(Deserialize)]
        struct Owner {
            login: String,
        }
        let page = self
            .authed_get::<RepoList>(
                installation_id,
                "https://api.github.com/installation/repositories?per_page=100".to_string(),
            )
            .await?;
        Ok(page
            .repositories
            .into_iter()
            .map(|repo| GitHubRepository {
                repository_id: repo.id,
                owner: repo.owner.login,
                name: repo.name,
                full_name: repo.full_name,
                default_branch: repo.default_branch,
                private: repo.private,
                archived: repo.archived,
            })
            .collect())
    }

    async fn get_tree(
        &self,
        installation_id: i64,
        owner: &str,
        repo: &str,
        branch: &str,
    ) -> Result<(String, Vec<GitHubTreeEntry>, bool), GitHubClientError> {
        #[derive(Deserialize)]
        struct TreeResponse {
            sha: String,
            tree: Vec<Entry>,
            truncated: bool,
        }
        #[derive(Deserialize)]
        struct Entry {
            path: String,
            sha: String,
            #[serde(rename = "type")]
            kind: String,
            size: Option<i64>,
        }
        let tree = self
            .authed_get::<TreeResponse>(
                installation_id,
                format!(
                    "https://api.github.com/repos/{owner}/{repo}/git/trees/{branch}?recursive=1"
                ),
            )
            .await?;
        Ok((
            tree.sha,
            tree.tree
                .into_iter()
                .map(|entry| GitHubTreeEntry {
                    path: entry.path,
                    sha: entry.sha,
                    kind: entry.kind,
                    size: entry.size,
                })
                .collect(),
            tree.truncated,
        ))
    }

    async fn get_file(
        &self,
        installation_id: i64,
        owner: &str,
        repo: &str,
        path: &str,
        reference: &str,
    ) -> Result<GitHubFile, GitHubClientError> {
        #[derive(Deserialize)]
        struct ContentResponse {
            path: String,
            sha: String,
            content: String,
            encoding: String,
        }
        let file = self
            .authed_get::<ContentResponse>(
                installation_id,
                format!(
                    "https://api.github.com/repos/{owner}/{repo}/contents/{path}?ref={reference}"
                ),
            )
            .await?;
        if file.encoding != "base64" {
            return Err(GitHubClientError::Parse);
        }
        use base64::Engine as _;
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(file.content.replace('\n', ""))
            .map_err(|_| GitHubClientError::Parse)?;
        Ok(GitHubFile {
            path: file.path,
            sha: file.sha,
            bytes,
        })
    }

    async fn create_draft_pr(
        &self,
        request: GitHubDraftPrRequest,
    ) -> Result<GitHubPullRequest, GitHubClientError> {
        let GitHubDraftPrRequest {
            installation_id,
            owner,
            repo,
            base_branch,
            base_sha,
            branch_name,
            changes,
            title,
            body,
        } = request;
        #[derive(Deserialize)]
        struct RefResponse {
            object: RefObject,
        }
        #[derive(Deserialize)]
        struct RefObject {
            sha: String,
        }
        #[derive(Deserialize)]
        struct CommitResponse {
            tree: RefObject,
        }
        #[derive(Deserialize)]
        struct ShaResponse {
            sha: String,
        }
        #[derive(Deserialize)]
        struct PullResponse {
            number: i64,
            html_url: String,
        }
        let token = self.installation_token(installation_id).await?;
        let ref_url =
            format!("https://api.github.com/repos/{owner}/{repo}/git/ref/heads/{base_branch}");
        let base_ref = self
            .authed_get::<RefResponse>(installation_id, ref_url)
            .await?;
        if base_ref.object.sha != base_sha {
            return Err(GitHubClientError::Conflict);
        }
        let base_commit = self
            .authed_get::<CommitResponse>(
                installation_id,
                format!("https://api.github.com/repos/{owner}/{repo}/git/commits/{base_sha}"),
            )
            .await?;

        let mut tree_entries = Vec::with_capacity(changes.len());
        for change in changes {
            let blob = post_json::<ShaResponse>(
                &self.http,
                &token,
                &format!("https://api.github.com/repos/{owner}/{repo}/git/blobs"),
                &serde_json::json!({
                    "content": change.replacement,
                    "encoding": "utf-8",
                }),
            )
            .await?;
            tree_entries.push(serde_json::json!({
                "path": change.path,
                "mode": "100644",
                "type": "blob",
                "sha": blob.sha,
            }));
        }
        let new_tree = post_json::<ShaResponse>(
            &self.http,
            &token,
            &format!("https://api.github.com/repos/{owner}/{repo}/git/trees"),
            &serde_json::json!({
                "base_tree": base_commit.tree.sha,
                "tree": tree_entries,
            }),
        )
        .await?;
        let new_commit = post_json::<ShaResponse>(
            &self.http,
            &token,
            &format!("https://api.github.com/repos/{owner}/{repo}/git/commits"),
            &serde_json::json!({
                "message": "Integrate TrustLoopGuard",
                "tree": new_tree.sha,
                "parents": [base_sha],
            }),
        )
        .await?;
        let ref_res = post_json::<serde_json::Value>(
            &self.http,
            &token,
            &format!("https://api.github.com/repos/{owner}/{repo}/git/refs"),
            &serde_json::json!({
                "ref": format!("refs/heads/{branch_name}"),
                "sha": new_commit.sha,
            }),
        )
        .await;
        if let Err(GitHubClientError::Conflict) = ref_res {
            tracing::info!(branch_name, "github integration branch already exists");
        } else {
            ref_res?;
        }
        let pr = post_json::<PullResponse>(
            &self.http,
            &token,
            &format!("https://api.github.com/repos/{owner}/{repo}/pulls"),
            &serde_json::json!({
                    "title": title,
                "head": branch_name,
                "base": base_branch,
                "body": body,
                "draft": true,
            }),
        )
        .await?;
        Ok(GitHubPullRequest {
            number: pr.number,
            url: pr.html_url,
            branch_name,
            commit_sha: new_commit.sha,
        })
    }
}

async fn post_json<T: for<'de> Deserialize<'de>>(
    http: &reqwest::Client,
    token: &str,
    url: &str,
    body: &serde_json::Value,
) -> Result<T, GitHubClientError> {
    let res = http
        .post(url)
        .headers(base_headers())
        .header(AUTHORIZATION, format!("Bearer {token}"))
        .json(body)
        .send()
        .await
        .map_err(|_| GitHubClientError::Transport)?;
    if !res.status().is_success() {
        return Err(status_error(res.status().as_u16()));
    }
    res.json::<T>().await.map_err(|_| GitHubClientError::Parse)
}

fn base_headers() -> reqwest::header::HeaderMap {
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert(ACCEPT, ACCEPT_JSON.parse().expect("valid accept header"));
    headers.insert(
        USER_AGENT,
        USER_AGENT_VALUE.parse().expect("valid user agent"),
    );
    headers.insert(
        API_VERSION_HEADER,
        API_VERSION.parse().expect("valid API version"),
    );
    headers
}

fn status_error(status: u16) -> GitHubClientError {
    match status {
        404 => GitHubClientError::NotFound,
        409 => GitHubClientError::Conflict,
        401 | 403 => GitHubClientError::Auth,
        status => GitHubClientError::Status { status },
    }
}
