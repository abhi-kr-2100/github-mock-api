use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::util::{LoadError, load_json_from_file};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct RepositoryOwner {
    pub login: String,
    pub id: u64,
    pub node_id: String,
    pub avatar_url: String,
    pub gravatar_id: String,
    pub url: String,
    pub html_url: String,
    pub followers_url: String,
    pub following_url: String,
    pub gists_url: String,
    pub starred_url: String,
    pub subscriptions_url: String,
    pub organizations_url: String,
    pub repos_url: String,
    pub events_url: String,
    pub received_events_url: String,
    #[serde(rename = "type")]
    pub owner_type: String,
    pub site_admin: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_view_type: Option<String>,
}

impl RepositoryOwner {
    pub(crate) fn new(login: &str) -> Self {
        let id = crate::util::hash(&format!("owner:{login}"));
        Self {
            login: login.to_string(),
            id,
            node_id: format!("mock_node_id_{id}"),
            avatar_url: format!("https://avatars.githubusercontent.com/u/{id}?v=4"),
            gravatar_id: String::new(),
            url: format!("https://api.github.com/users/{login}"),
            html_url: format!("https://github.com/{login}"),
            followers_url: format!("https://api.github.com/users/{login}/followers"),
            following_url: format!("https://api.github.com/users/{login}/following{{/other_user}}"),
            gists_url: format!("https://api.github.com/users/{login}/gists{{/gist_id}}"),
            starred_url: format!("https://api.github.com/users/{login}/starred{{/owner}}{{/repo}}"),
            subscriptions_url: format!("https://api.github.com/users/{login}/subscriptions"),
            organizations_url: format!("https://api.github.com/users/{login}/orgs"),
            repos_url: format!("https://api.github.com/users/{login}/repos"),
            events_url: format!("https://api.github.com/users/{login}/events{{/privacy}}"),
            received_events_url: format!("https://api.github.com/users/{login}/received_events"),
            owner_type: "User".to_string(),
            site_admin: false,
            user_view_type: Some("public".to_string()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct RepositoryLicense {
    pub key: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spdx_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    pub node_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Repository {
    pub id: u64,
    pub node_id: String,
    pub name: String,
    pub full_name: String,
    pub owner: RepositoryOwner,
    pub private: bool,
    pub html_url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub fork: bool,
    pub url: String,
    pub archive_url: String,
    pub assignees_url: String,
    pub blobs_url: String,
    pub branches_url: String,
    pub collaborators_url: String,
    pub comments_url: String,
    pub commits_url: String,
    pub compare_url: String,
    pub contents_url: String,
    pub contributors_url: String,
    pub deployments_url: String,
    pub downloads_url: String,
    pub events_url: String,
    pub forks_url: String,
    pub git_commits_url: String,
    pub git_refs_url: String,
    pub git_tags_url: String,
    pub git_url: String,
    pub issue_comment_url: String,
    pub issue_events_url: String,
    pub issues_url: String,
    pub keys_url: String,
    pub labels_url: String,
    pub languages_url: String,
    pub merges_url: String,
    pub milestones_url: String,
    pub notifications_url: String,
    pub pulls_url: String,
    pub releases_url: String,
    pub ssh_url: String,
    pub stargazers_url: String,
    pub statuses_url: String,
    pub subscribers_url: String,
    pub subscription_url: String,
    pub tags_url: String,
    pub teams_url: String,
    pub trees_url: String,
    pub clone_url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mirror_url: Option<String>,
    pub hooks_url: String,
    pub svn_url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub homepage: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    pub forks_count: u64,
    pub forks: u64,
    pub stargazers_count: u64,
    pub watchers_count: u64,
    pub watchers: u64,
    pub size: u64,
    pub default_branch: String,
    pub open_issues_count: u64,
    pub open_issues: u64,
    #[serde(default)]
    pub subscribers_count: u64,
    #[serde(default)]
    pub network_count: u64,
    pub is_template: bool,
    pub topics: Vec<String>,
    pub has_issues: bool,
    pub has_projects: bool,
    pub has_wiki: bool,
    pub has_pages: bool,
    pub has_downloads: bool,
    pub has_discussions: bool,
    pub archived: bool,
    pub disabled: bool,
    pub visibility: String,
    pub pushed_at: String,
    pub created_at: String,
    pub updated_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub license: Option<RepositoryLicense>,
    pub allow_forking: bool,
    pub web_commit_signoff_required: bool,
}

impl Repository {
    /// Load repositories from a JSON file.
    ///
    /// The JSON file should contain an array of repository objects as returned by the GitHub API.
    pub fn load_from_file(path: impl AsRef<Path>) -> Result<Vec<Self>, LoadError> {
        load_json_from_file(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_from_file_happy_path() -> Result<(), Box<dyn std::error::Error>> {
        let repos = Repository::load_from_file("testing/data/repositories.json")?;

        assert_eq!(repos.len(), 30);
        assert_eq!(repos[0].name, "arxiv-sanity-lite");

        Ok(())
    }

    #[test]
    fn test_load_from_file_io_error() {
        let result = Repository::load_from_file("non_existent_file.json");
        assert!(matches!(result, Err(LoadError::Io { .. })));
    }

    #[test]
    fn test_load_from_file_json_error() {
        let result = Repository::load_from_file("testing/data/repositories_invalid.json");
        assert!(matches!(result, Err(LoadError::Json { .. })));
    }

    #[test]
    fn test_repository_owner_url_serialization() {
        let owner = RepositoryOwner::new("octocat");
        let json = serde_json::to_string(&owner).unwrap();
        let val: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert!(val.get("url").is_some());
        assert!(val.get("url").unwrap().is_string());
        assert_eq!(
            val.get("url").unwrap().as_str().unwrap(),
            "https://api.github.com/users/octocat"
        );
    }

    #[test]
    fn test_repository_owner_new_fields() {
        let owner = RepositoryOwner::new("octocat");
        let json = serde_json::to_string(&owner).unwrap();
        let val: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert_eq!(
            val["followers_url"],
            "https://api.github.com/users/octocat/followers"
        );
        assert_eq!(
            val["following_url"],
            "https://api.github.com/users/octocat/following{/other_user}"
        );
        assert_eq!(
            val["gists_url"],
            "https://api.github.com/users/octocat/gists{/gist_id}"
        );
        assert_eq!(
            val["starred_url"],
            "https://api.github.com/users/octocat/starred{/owner}{/repo}"
        );
        assert_eq!(
            val["subscriptions_url"],
            "https://api.github.com/users/octocat/subscriptions"
        );
        assert_eq!(
            val["organizations_url"],
            "https://api.github.com/users/octocat/orgs"
        );
        assert_eq!(
            val["events_url"],
            "https://api.github.com/users/octocat/events{/privacy}"
        );
        assert_eq!(
            val["received_events_url"],
            "https://api.github.com/users/octocat/received_events"
        );
        assert_eq!(val["site_admin"], false);
        assert_eq!(val["user_view_type"], "public");
    }
}
