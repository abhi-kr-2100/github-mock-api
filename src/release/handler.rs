use crate::api::{ApiError, ApiResponse};
use crate::release::types::Release;
use crate::AppState;
use crate::util::Pagination;

pub async fn list_releases(
    owner: String,
    repo: String,
    pagination: Pagination,
    state: AppState,
) -> ApiResponse<Vec<Release>> {
    let key = (owner.to_lowercase(), repo.to_lowercase());
    let releases_map = state.releases.read().await;

    let releases = match releases_map.get(&key) {
        Some(r) => r,
        None => return ApiResponse::Ok(Vec::new()),
    };

    let mut releases = releases.clone();
    // Sort by created_at descending (latest first)
    releases.sort_by(|a, b| b.created_at.cmp(&a.created_at));

    ApiResponse::Ok(crate::util::paginate(&releases, pagination))
}

pub async fn get_release(owner: String, repo: String, id: u64, state: AppState) -> ApiResponse<Release> {
    let key = (owner.to_lowercase(), repo.to_lowercase());
    let releases_map = state.releases.read().await;
    let releases = match releases_map.get(&key) {
        Some(r) => r,
        None => {
            return ApiResponse::Error(ApiError::not_found(
                "Not Found",
                "https://docs.github.com/rest/releases/releases#get-a-release",
            ))
        }
    };

    match releases.iter().find(|r| r.id == id) {
        Some(release) => ApiResponse::Ok(release.clone()),
        None => ApiResponse::Error(ApiError::not_found(
            "Not Found",
            "https://docs.github.com/rest/releases/releases#get-a-release",
        )),
    }
}

pub async fn get_release_by_tag(
    owner: String,
    repo: String,
    tag: String,
    state: AppState,
) -> ApiResponse<Release> {
    let key = (owner.to_lowercase(), repo.to_lowercase());
    let releases_map = state.releases.read().await;
    let releases = match releases_map.get(&key) {
        Some(r) => r,
        None => {
            return ApiResponse::Error(ApiError::not_found(
                "Not Found",
                "https://docs.github.com/rest/releases/releases#get-a-release-by-tag-name",
            ))
        }
    };

    match releases.iter().find(|r| r.tag_name == tag) {
        Some(release) => ApiResponse::Ok(release.clone()),
        None => ApiResponse::Error(ApiError::not_found(
            "Not Found",
            "https://docs.github.com/rest/releases/releases#get-a-release-by-tag-name",
        )),
    }
}

pub async fn get_latest_release(owner: String, repo: String, state: AppState) -> ApiResponse<Release> {
    let key = (owner.to_lowercase(), repo.to_lowercase());
    let releases_map = state.releases.read().await;
    let releases = match releases_map.get(&key) {
        Some(r) => r,
        None => {
            return ApiResponse::Error(ApiError::not_found(
                "Not Found",
                "https://docs.github.com/rest/releases/releases#get-the-latest-release",
            ))
        }
    };

    // Latest release is the newest non-draft, non-prerelease
    let mut candidates: Vec<_> = releases
        .iter()
        .filter(|r| !r.draft && !r.prerelease)
        .cloned()
        .collect();
    candidates.sort_by(|a, b| b.created_at.cmp(&a.created_at));

    match candidates.into_iter().next() {
        Some(release) => ApiResponse::Ok(release),
        None => ApiResponse::Error(ApiError::not_found(
            "Not Found",
            "https://docs.github.com/rest/releases/releases#get-the-latest-release",
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::MockServer;
    use axum::http::StatusCode;
    use reqwest::Client;

    #[tokio::test]
    async fn test_list_releases_returns_releases() {
        let server = MockServer::start().await.unwrap();

        let release = Release::new("owner", "repo", "v1.0.0");
        server.add_release(release).await;

        let client = Client::new();
        let response = client
            .get(server.uri() + "/repos/owner/repo/releases")
            .send()
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let releases: Vec<Release> = response.json().await.unwrap();
        assert_eq!(releases.len(), 1);
        assert_eq!(releases[0].tag_name, "v1.0.0");
    }

    #[tokio::test]
    async fn test_get_release_by_id() {
        let server = MockServer::start().await.unwrap();
        let release = Release::new("owner", "repo", "v1.0.0");
        let id = release.id;
        server.add_release(release).await;

        let client = Client::new();
        let response = client
            .get(server.uri() + &format!("/repos/owner/repo/releases/{}", id))
            .send()
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let release: Release = response.json().await.unwrap();
        assert_eq!(release.id, id);
    }

    #[tokio::test]
    async fn test_get_release_by_tag() {
        let server = MockServer::start().await.unwrap();
        let release = Release::new("owner", "repo", "v1.0.0");
        server.add_release(release).await;

        let client = Client::new();
        let response = client
            .get(server.uri() + "/repos/owner/repo/releases/tags/v1.0.0")
            .send()
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let release: Release = response.json().await.unwrap();
        assert_eq!(release.tag_name, "v1.0.0");
    }

    #[tokio::test]
    async fn test_get_release_not_found() {
        let server = MockServer::start().await.unwrap();
        let client = Client::new();
        let response = client
            .get(server.uri() + "/repos/owner/repo/releases/123")
            .send()
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_list_releases_empty() {
        let server = MockServer::start().await.unwrap();
        let client = Client::new();
        let response = client
            .get(server.uri() + "/repos/owner/repo/releases")
            .send()
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let releases: Vec<Release> = response.json().await.unwrap();
        assert!(releases.is_empty());
    }

    #[tokio::test]
    async fn test_get_latest_release() {
        let server = MockServer::start().await.unwrap();

        server.add_release(Release::new("owner", "repo", "v1.0.0").created_at("2024-01-01T00:00:00Z")).await;
        server.add_release(Release::new("owner", "repo", "v1.1.0").created_at("2024-02-01T00:00:00Z")).await;

        let client = Client::new();
        let response = client
            .get(server.uri() + "/repos/owner/repo/releases/latest")
            .send()
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let release: Release = response.json().await.unwrap();
        assert_eq!(release.tag_name, "v1.1.0");
    }

    #[tokio::test]
    async fn test_list_releases_pagination() {
        let server = MockServer::start().await.unwrap();

        for i in 1..=35 {
            server.add_release(Release::new("owner", "repo", &format!("v{}", i))).await;
        }

        let client = reqwest::Client::new();

        // Default (30 items)
        let resp = client
            .get(server.uri() + "/repos/owner/repo/releases")
            .send()
            .await
            .unwrap();
        let body: Vec<serde_json::Value> = resp.json().await.unwrap();
        assert_eq!(body.len(), 30);

        // Page 2 (Remaining 5 items)
        let resp = client
            .get(server.uri() + "/repos/owner/repo/releases?page=2")
            .send()
            .await
            .unwrap();
        let body: Vec<serde_json::Value> = resp.json().await.unwrap();
        assert_eq!(body.len(), 5);
    }
}
