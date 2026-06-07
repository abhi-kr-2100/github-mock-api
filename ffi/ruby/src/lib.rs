use std::sync::Mutex;

use ::github_mock_api::{
    Asset as RustAsset, Commit as RustCommit, Error as MockApiError,
    MockBehavior as RustMockBehavior, MockError as RustMockError, MockServer as RustMockServer,
    Release as RustRelease, Repository as RustRepository,
};
use github_mock_api_ffi_common::{CommonError, parse_host, runtime};
use magnus::{Error, IntoValue, Ruby, function, method, prelude::*, wrap};

fn runtime_error(ruby: &Ruby, err: CommonError) -> Error {
    match err {
        CommonError::Io => Error::new(ruby.exception_io_error(), "failed to initialize runtime"),
        CommonError::InvalidHost => Error::new(ruby.exception_arg_error(), "invalid host"),
        CommonError::Shutdown => Error::new(ruby.exception_runtime_error(), "shutdown error"),
        CommonError::Join => Error::new(ruby.exception_runtime_error(), "join error"),
        CommonError::Conflict => {
            Error::new(ruby.exception_runtime_error(), "mock behavior conflict")
        }
        CommonError::DataLoad => Error::new(ruby.exception_runtime_error(), "data load error"),
    }
}

fn mock_api_error(ruby: &Ruby, err: MockApiError) -> Error {
    match err {
        MockApiError::Io(err) => Error::new(ruby.exception_io_error(), err.to_string()),
        MockApiError::ShutdownError(err) => {
            Error::new(ruby.exception_runtime_error(), err.to_string())
        }
        MockApiError::JoinError(err) => Error::new(ruby.exception_runtime_error(), err.to_string()),
        MockApiError::Conflict(err) => Error::new(ruby.exception_runtime_error(), err),
    }
}

fn lock_error(ruby: &Ruby) -> Error {
    Error::new(ruby.exception_runtime_error(), "mock server lock poisoned")
}

#[wrap(class = "GitHubMockAPI::MockServer", free_immediately, size)]
struct MockServer {
    server: Mutex<Option<RustMockServer>>,
    uri: String,
}

impl MockServer {
    fn new(server: RustMockServer) -> Self {
        let uri = server.uri();
        Self {
            server: Mutex::new(Some(server)),
            uri,
        }
    }

    fn start(ruby: &Ruby) -> Result<Self, Error> {
        let server = runtime()
            .map_err(|err| runtime_error(ruby, err))?
            .block_on(RustMockServer::start())
            .map_err(|err| mock_api_error(ruby, err))?;
        Ok(Self::new(server))
    }

    fn start_on(ruby: &Ruby, host: String, port: u16) -> Result<Self, Error> {
        let host = parse_host(&host).map_err(|e| runtime_error(ruby, e))?;
        let server = runtime()
            .map_err(|err| runtime_error(ruby, err))?
            .block_on(RustMockServer::start_on(host, port))
            .map_err(|err| mock_api_error(ruby, err))?;
        Ok(Self::new(server))
    }

    fn uri(_ruby: &Ruby, rb_self: &Self) -> Result<String, Error> {
        Ok(rb_self.uri.clone())
    }

    fn stop(ruby: &Ruby, rb_self: &Self) -> Result<(), Error> {
        let server = rb_self.server.lock().map_err(|_| lock_error(ruby))?.take();

        if let Some(mut server) = server {
            runtime()
                .map_err(|err| runtime_error(ruby, err))?
                .block_on(server.stop())
                .map_err(|err| mock_api_error(ruby, err))?;
        }

        Ok(())
    }

    fn add_repository(ruby: &Ruby, rb_self: &Self, repository: &Repository) -> Result<(), Error> {
        let lock = rb_self.server.lock().map_err(|_| lock_error(ruby))?;
        if let Some(ref server) = *lock {
            runtime()
                .map_err(|err| runtime_error(ruby, err))?
                .block_on(server.add_repository(repository.inner.clone()));
        }
        Ok(())
    }

    fn add_release(
        ruby: &Ruby,
        rb_self: &Self,
        owner: String,
        repo: String,
        release: &Release,
    ) -> Result<(), Error> {
        let lock = rb_self.server.lock().map_err(|_| lock_error(ruby))?;
        if let Some(ref server) = *lock {
            runtime()
                .map_err(|err| runtime_error(ruby, err))?
                .block_on(server.add_release(&owner, &repo, release.inner.clone()));
        }
        Ok(())
    }

    fn add_commit(
        ruby: &Ruby,
        rb_self: &Self,
        owner: String,
        repo: String,
        commit: &Commit,
    ) -> Result<(), Error> {
        let lock = rb_self.server.lock().map_err(|_| lock_error(ruby))?;
        if let Some(ref server) = *lock {
            runtime()
                .map_err(|err| runtime_error(ruby, err))?
                .block_on(server.add_commit(&owner, &repo, commit.inner.clone()));
        }
        Ok(())
    }

    fn add_asset(
        ruby: &Ruby,
        rb_self: &Self,
        owner: String,
        repo: String,
        tag: String,
        asset: &Asset,
    ) -> Result<(), Error> {
        let lock = rb_self.server.lock().map_err(|_| lock_error(ruby))?;
        if let Some(ref server) = *lock {
            runtime()
                .map_err(|err| runtime_error(ruby, err))?
                .block_on(server.add_asset(&owner, &repo, &tag, asset.inner.clone()));
        }
        Ok(())
    }

    fn add_mock_behavior(ruby: &Ruby, rb_self: &Self, behavior: &MockBehavior) -> Result<(), Error> {
        let lock = rb_self.server.lock().map_err(|_| lock_error(ruby))?;
        if let Some(ref server) = *lock {
            runtime()
                .map_err(|err| runtime_error(ruby, err))?
                .block_on(server.add_mock_behavior(behavior.inner.clone()))
                .map_err(|err| mock_api_error(ruby, err))?;
        }
        Ok(())
    }

    fn clear_all_mock_behaviors(ruby: &Ruby, rb_self: &Self) -> Result<(), Error> {
        let lock = rb_self.server.lock().map_err(|_| lock_error(ruby))?;
        if let Some(ref server) = *lock {
            runtime()
                .map_err(|err| runtime_error(ruby, err))?
                .block_on(server.clear_all_mock_behaviors());
        }
        Ok(())
    }
}

#[wrap(class = "GitHubMockAPI::Repository", free_immediately, size)]
struct Repository {
    inner: RustRepository,
}

impl Repository {
    fn new(owner: String, name: String) -> Self {
        Self {
            inner: RustRepository::new(&owner, &name),
        }
    }

    fn name(&self) -> String {
        self.inner.name.clone()
    }

    fn description(rb_self: &Self, description: String) -> Self {
        Self {
            inner: rb_self.inner.clone().description(description),
        }
    }

    fn private(rb_self: &Self, private: bool) -> Self {
        Self {
            inner: rb_self.inner.clone().private(private),
        }
    }

    fn stargazers_count(rb_self: &Self, count: u64) -> Self {
        Self {
            inner: rb_self.inner.clone().stargazers_count(count),
        }
    }

    fn default_branch(rb_self: &Self, branch: String) -> Self {
        Self {
            inner: rb_self.inner.clone().default_branch(branch),
        }
    }
}

#[wrap(class = "GitHubMockAPI::Release", free_immediately, size)]
struct Release {
    inner: RustRelease,
}

impl Release {
    fn new(owner: String, repo: String, tag_name: String) -> Self {
        Self {
            inner: RustRelease::new(&owner, &repo, &tag_name),
        }
    }

    fn name(rb_self: &Self, name: String) -> Self {
        Self {
            inner: rb_self.inner.clone().name(name),
        }
    }

    fn body(rb_self: &Self, body: String) -> Self {
        Self {
            inner: rb_self.inner.clone().body(body),
        }
    }

    fn draft(rb_self: &Self, draft: bool) -> Self {
        Self {
            inner: rb_self.inner.clone().draft(draft),
        }
    }

    fn prerelease(rb_self: &Self, prerelease: bool) -> Self {
        Self {
            inner: rb_self.inner.clone().prerelease(prerelease),
        }
    }
}

#[wrap(class = "GitHubMockAPI::Commit", free_immediately, size)]
struct Commit {
    inner: RustCommit,
}

impl Commit {
    fn new(owner: String, repo: String) -> Self {
        Self {
            inner: RustCommit::new(&owner, &repo),
        }
    }

    fn sha(rb_self: &Self, sha: String) -> Self {
        Self {
            inner: rb_self.inner.clone().sha(sha),
        }
    }

    fn message(rb_self: &Self, message: String) -> Self {
        Self {
            inner: rb_self.inner.clone().message(message),
        }
    }
}

#[wrap(class = "GitHubMockAPI::Asset", free_immediately, size)]
struct Asset {
    inner: RustAsset,
}

impl Asset {
    fn from_bytes(name: String, bytes: Vec<u8>, content_type: String) -> Self {
        Self {
            inner: RustAsset::from_bytes(name, bytes, content_type),
        }
    }

    fn from_path(name: String, path: String, content_type: String) -> Self {
        Self {
            inner: RustAsset::from_path(name, path, content_type),
        }
    }
}

#[wrap(class = "GitHubMockAPI::MockBehavior", free_immediately, size)]
struct MockBehavior {
    inner: RustMockBehavior,
}

impl MockBehavior {
    fn error(ruby: &Ruby, error: i32) -> Result<Self, Error> {
        let rust_error = match error {
            0 => RustMockError::InternalServerError,
            1 => RustMockError::RateLimitExceeded,
            _ => return Err(Error::new(ruby.exception_arg_error(), "invalid error code")),
        };
        Ok(Self {
            inner: RustMockBehavior::builder().error(rust_error).build(),
        })
    }
}

fn load_repos(ruby: &Ruby, path: String) -> Result<magnus::Value, Error> {
    let repos = RustRepository::load_from_file(path)
        .map_err(|e| Error::new(ruby.exception_runtime_error(), e.to_string()))?;
    let ary = ruby.ary_new();
    for r in repos {
        ary.push(Repository { inner: r })?;
    }
    Ok(ary.into_value())
}

fn load_releases(ruby: &Ruby, path: String, owner: String, repo: String) -> Result<magnus::Value, Error> {
    let releases = RustRelease::load_from_file(path, &owner, &repo)
        .map_err(|e| Error::new(ruby.exception_runtime_error(), e.to_string()))?;
    let ary = ruby.ary_new();
    for r in releases {
        ary.push(Release { inner: r })?;
    }
    Ok(ary.into_value())
}

fn load_commits(ruby: &Ruby, path: String, owner: String, repo: String) -> Result<magnus::Value, Error> {
    let commits = RustCommit::load_from_file(path, &owner, &repo)
        .map_err(|e| Error::new(ruby.exception_runtime_error(), e.to_string()))?;
    let ary = ruby.ary_new();
    for r in commits {
        ary.push(Commit { inner: r })?;
    }
    Ok(ary.into_value())
}

#[magnus::init]
pub fn init(ruby: &Ruby) -> Result<(), Error> {
    let module = ruby.define_module("GitHubMockAPI")?;

    let class = module.define_class("MockServer", ruby.class_object())?;
    class.define_singleton_method("start", function!(MockServer::start, 0))?;
    class.define_singleton_method("start_on", function!(MockServer::start_on, 2))?;
    class.define_method("uri", method!(MockServer::uri, 0))?;
    class.define_method("stop", method!(MockServer::stop, 0))?;
    class.define_method("add_repository", method!(MockServer::add_repository, 1))?;
    class.define_method("add_release", method!(MockServer::add_release, 3))?;
    class.define_method("add_commit", method!(MockServer::add_commit, 3))?;
    class.define_method("add_asset", method!(MockServer::add_asset, 4))?;
    class.define_method("add_mock_behavior", method!(MockServer::add_mock_behavior, 1))?;
    class.define_method("clear_all_mock_behaviors", method!(MockServer::clear_all_mock_behaviors, 0))?;

    let repo_class = module.define_class("Repository", ruby.class_object())?;
    repo_class.define_singleton_method("new", function!(Repository::new, 2))?;
    repo_class.define_singleton_method("load_from_file", function!(load_repos, 1))?;
    repo_class.define_method("name", method!(Repository::name, 0))?;
    repo_class.define_method("description", method!(Repository::description, 1))?;
    repo_class.define_method("private", method!(Repository::private, 1))?;
    repo_class.define_method("stargazers_count", method!(Repository::stargazers_count, 1))?;
    repo_class.define_method("default_branch", method!(Repository::default_branch, 1))?;

    let release_class = module.define_class("Release", ruby.class_object())?;
    release_class.define_singleton_method("new", function!(Release::new, 3))?;
    release_class.define_singleton_method("load_from_file", function!(load_releases, 3))?;
    release_class.define_method("name", method!(Release::name, 1))?;
    release_class.define_method("body", method!(Release::body, 1))?;
    release_class.define_method("draft", method!(Release::draft, 1))?;
    release_class.define_method("prerelease", method!(Release::prerelease, 1))?;

    let commit_class = module.define_class("Commit", ruby.class_object())?;
    commit_class.define_singleton_method("new", function!(Commit::new, 2))?;
    commit_class.define_singleton_method("load_from_file", function!(load_commits, 3))?;
    commit_class.define_method("sha", method!(Commit::sha, 1))?;
    commit_class.define_method("message", method!(Commit::message, 1))?;

    let asset_class = module.define_class("Asset", ruby.class_object())?;
    asset_class.define_singleton_method("from_bytes", function!(Asset::from_bytes, 3))?;
    asset_class.define_singleton_method("from_path", function!(Asset::from_path, 3))?;

    let behavior_class = module.define_class("MockBehavior", ruby.class_object())?;
    behavior_class.define_singleton_method("error", function!(MockBehavior::error, 1))?;

    module.const_set("INTERNAL_SERVER_ERROR", 0)?;
    module.const_set("RATE_LIMIT_EXCEEDED", 1)?;

    Ok(())
}
