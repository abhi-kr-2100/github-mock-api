use std::sync::Mutex;

use github_mock_api::{
    Asset as RustAsset, Commit as RustCommit, Error as MockApiError,
    MockBehavior as RustMockBehavior, MockError, MockServer as RustMockServer,
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

fn server_stopped_error(ruby: &Ruby) -> Error {
    ruby.define_module("GitHubMockAPI")
        .and_then(|m| m.const_get::<_, magnus::Value>("ServerStoppedError"))
        .ok()
        .and_then(magnus::ExceptionClass::from_value)
        .map(|exc| Error::new(exc, "server has been stopped"))
        .unwrap_or_else(|| Error::new(ruby.exception_runtime_error(), "server has been stopped"))
}

#[wrap(class = "GitHubMockAPI::Repository", free_immediately, size)]
struct Repository {
    inner: Mutex<RustRepository>,
}

impl Repository {
    fn new(owner: String, name: String) -> Self {
        Self {
            inner: Mutex::new(RustRepository::new(&owner, &name)),
        }
    }

    fn description(
        ruby: &Ruby,
        rb_self: magnus::typed_data::Obj<Self>,
        description: String,
    ) -> Result<magnus::Value, Error> {
        let mut inner = rb_self.inner.lock().map_err(|_| lock_error(ruby))?;
        *inner = inner.clone().description(description);
        Ok(rb_self.into_value_with(ruby))
    }

    fn clear_description(
        ruby: &Ruby,
        rb_self: magnus::typed_data::Obj<Self>,
    ) -> Result<magnus::Value, Error> {
        let mut inner = rb_self.inner.lock().map_err(|_| lock_error(ruby))?;
        *inner = inner.clone().clear_description();
        Ok(rb_self.into_value_with(ruby))
    }

    fn private(
        ruby: &Ruby,
        rb_self: magnus::typed_data::Obj<Self>,
        private: bool,
    ) -> Result<magnus::Value, Error> {
        let mut inner = rb_self.inner.lock().map_err(|_| lock_error(ruby))?;
        *inner = inner.clone().private(private);
        Ok(rb_self.into_value_with(ruby))
    }

    fn stargazers_count(
        ruby: &Ruby,
        rb_self: magnus::typed_data::Obj<Self>,
        count: u64,
    ) -> Result<magnus::Value, Error> {
        let mut inner = rb_self.inner.lock().map_err(|_| lock_error(ruby))?;
        *inner = inner.clone().stargazers_count(count);
        Ok(rb_self.into_value_with(ruby))
    }

    fn default_branch(
        ruby: &Ruby,
        rb_self: magnus::typed_data::Obj<Self>,
        branch: String,
    ) -> Result<magnus::Value, Error> {
        let mut inner = rb_self.inner.lock().map_err(|_| lock_error(ruby))?;
        *inner = inner.clone().default_branch(branch);
        Ok(rb_self.into_value_with(ruby))
    }

    fn subscribers_count(
        ruby: &Ruby,
        rb_self: magnus::typed_data::Obj<Self>,
        count: u64,
    ) -> Result<magnus::Value, Error> {
        let mut inner = rb_self.inner.lock().map_err(|_| lock_error(ruby))?;
        *inner = inner.clone().subscribers_count(count);
        Ok(rb_self.into_value_with(ruby))
    }

    fn network_count(
        ruby: &Ruby,
        rb_self: magnus::typed_data::Obj<Self>,
        count: u64,
    ) -> Result<magnus::Value, Error> {
        let mut inner = rb_self.inner.lock().map_err(|_| lock_error(ruby))?;
        *inner = inner.clone().network_count(count);
        Ok(rb_self.into_value_with(ruby))
    }
}

#[wrap(class = "GitHubMockAPI::Commit", free_immediately, size)]
struct Commit {
    inner: Mutex<RustCommit>,
    owner: String,
    repo: String,
}

impl Commit {
    fn new(owner: String, repo: String) -> Self {
        Self {
            inner: Mutex::new(RustCommit::new(&owner, &repo)),
            owner,
            repo,
        }
    }

    fn sha(
        ruby: &Ruby,
        rb_self: magnus::typed_data::Obj<Self>,
        sha: String,
    ) -> Result<magnus::Value, Error> {
        let mut inner = rb_self.inner.lock().map_err(|_| lock_error(ruby))?;
        *inner = inner.clone().sha(sha);
        Ok(rb_self.into_value_with(ruby))
    }

    fn message(
        ruby: &Ruby,
        rb_self: magnus::typed_data::Obj<Self>,
        message: String,
    ) -> Result<magnus::Value, Error> {
        let mut inner = rb_self.inner.lock().map_err(|_| lock_error(ruby))?;
        *inner = inner.clone().message(message);
        Ok(rb_self.into_value_with(ruby))
    }

    fn author_name(
        ruby: &Ruby,
        rb_self: magnus::typed_data::Obj<Self>,
        name: String,
    ) -> Result<magnus::Value, Error> {
        let mut inner = rb_self.inner.lock().map_err(|_| lock_error(ruby))?;
        *inner = inner.clone().author_name(name);
        Ok(rb_self.into_value_with(ruby))
    }

    fn author_email(
        ruby: &Ruby,
        rb_self: magnus::typed_data::Obj<Self>,
        email: String,
    ) -> Result<magnus::Value, Error> {
        let mut inner = rb_self.inner.lock().map_err(|_| lock_error(ruby))?;
        *inner = inner.clone().author_email(email);
        Ok(rb_self.into_value_with(ruby))
    }
}

#[wrap(class = "GitHubMockAPI::Asset", free_immediately, size)]
struct Asset {
    inner: Mutex<RustAsset>,
}

impl Asset {
    fn from_bytes(name: String, bytes: Vec<u8>, content_type: String) -> Self {
        Self {
            inner: Mutex::new(RustAsset::from_bytes(name, bytes, content_type)),
        }
    }

    fn from_file(name: String, path: String, content_type: String) -> Self {
        Self {
            inner: Mutex::new(RustAsset::from_path(name, path, content_type)),
        }
    }
}

#[wrap(class = "GitHubMockAPI::Release", free_immediately, size)]
struct Release {
    inner: Mutex<RustRelease>,
    owner: String,
    repo: String,
}

impl Release {
    fn new(owner: String, repo: String, tag_name: String) -> Self {
        Self {
            inner: Mutex::new(RustRelease::new(&owner, &repo, &tag_name)),
            owner,
            repo,
        }
    }

    fn name(
        ruby: &Ruby,
        rb_self: magnus::typed_data::Obj<Self>,
        name: String,
    ) -> Result<magnus::Value, Error> {
        let mut inner = rb_self.inner.lock().map_err(|_| lock_error(ruby))?;
        *inner = inner.clone().name(name);
        Ok(rb_self.into_value_with(ruby))
    }

    fn body(
        ruby: &Ruby,
        rb_self: magnus::typed_data::Obj<Self>,
        body: String,
    ) -> Result<magnus::Value, Error> {
        let mut inner = rb_self.inner.lock().map_err(|_| lock_error(ruby))?;
        *inner = inner.clone().body(body);
        Ok(rb_self.into_value_with(ruby))
    }

    fn target_commitish(
        ruby: &Ruby,
        rb_self: magnus::typed_data::Obj<Self>,
        commitish: String,
    ) -> Result<magnus::Value, Error> {
        let mut inner = rb_self.inner.lock().map_err(|_| lock_error(ruby))?;
        *inner = inner.clone().target_commitish(commitish);
        Ok(rb_self.into_value_with(ruby))
    }

    fn draft(
        ruby: &Ruby,
        rb_self: magnus::typed_data::Obj<Self>,
        draft: bool,
    ) -> Result<magnus::Value, Error> {
        let mut inner = rb_self.inner.lock().map_err(|_| lock_error(ruby))?;
        *inner = inner.clone().draft(draft);
        Ok(rb_self.into_value_with(ruby))
    }

    fn prerelease(
        ruby: &Ruby,
        rb_self: magnus::typed_data::Obj<Self>,
        prerelease: bool,
    ) -> Result<magnus::Value, Error> {
        let mut inner = rb_self.inner.lock().map_err(|_| lock_error(ruby))?;
        *inner = inner.clone().prerelease(prerelease);
        Ok(rb_self.into_value_with(ruby))
    }

    fn created_at(
        ruby: &Ruby,
        rb_self: magnus::typed_data::Obj<Self>,
        created_at: String,
    ) -> Result<magnus::Value, Error> {
        let mut inner = rb_self.inner.lock().map_err(|_| lock_error(ruby))?;
        *inner = inner.clone().created_at(created_at);
        Ok(rb_self.into_value_with(ruby))
    }
}

#[wrap(class = "GitHubMockAPI::MockBehavior", free_immediately, size)]
struct MockBehavior {
    inner: Mutex<RustMockBehavior>,
}

impl MockBehavior {
    fn new() -> Self {
        Self {
            inner: Mutex::new(RustMockBehavior::builder().build()),
        }
    }

    fn error(
        ruby: &Ruby,
        rb_self: magnus::typed_data::Obj<Self>,
        error: magnus::Symbol,
    ) -> Result<magnus::Value, Error> {
        let mock_error = if error == ruby.to_symbol("internal_server_error") {
            MockError::InternalServerError
        } else if error == ruby.to_symbol("rate_limit_exceeded") {
            MockError::RateLimitExceeded
        } else {
            return Err(Error::new(
                ruby.exception_arg_error(),
                format!("invalid mock error: {}", error),
            ));
        };

        let mut inner = rb_self.inner.lock().map_err(|_| lock_error(ruby))?;
        *inner = RustMockBehavior::builder().error(mock_error).build();

        Ok(rb_self.into_value_with(ruby))
    }
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

    fn add_mock_behavior(
        ruby: &Ruby,
        rb_self: &Self,
        behavior: &MockBehavior,
    ) -> Result<(), Error> {
        let mut lock = rb_self.server.lock().map_err(|_| lock_error(ruby))?;
        let server = lock.as_mut().ok_or_else(|| server_stopped_error(ruby))?;

        let behavior_inner = behavior.inner.lock().map_err(|_| lock_error(ruby))?.clone();

        runtime()
            .map_err(|err| runtime_error(ruby, err))?
            .block_on(server.add_mock_behavior(behavior_inner))
            .map_err(|err| mock_api_error(ruby, err))?;
        Ok(())
    }

    fn add_repository(ruby: &Ruby, rb_self: &Self, repository: &Repository) -> Result<(), Error> {
        let mut lock = rb_self.server.lock().map_err(|_| lock_error(ruby))?;
        let server = lock.as_mut().ok_or_else(|| server_stopped_error(ruby))?;

        let repo_inner = repository
            .inner
            .lock()
            .map_err(|_| lock_error(ruby))?
            .clone();

        runtime()
            .map_err(|err| runtime_error(ruby, err))?
            .block_on(server.add_repository(repo_inner));
        Ok(())
    }

    fn add_release(ruby: &Ruby, rb_self: &Self, release: &Release) -> Result<(), Error> {
        let mut lock = rb_self.server.lock().map_err(|_| lock_error(ruby))?;
        let server = lock.as_mut().ok_or_else(|| server_stopped_error(ruby))?;

        let release_inner = release.inner.lock().map_err(|_| lock_error(ruby))?.clone();

        runtime()
            .map_err(|err| runtime_error(ruby, err))?
            .block_on(server.add_release(&release.owner, &release.repo, release_inner));
        Ok(())
    }

    fn add_commit(ruby: &Ruby, rb_self: &Self, commit: &Commit) -> Result<(), Error> {
        let mut lock = rb_self.server.lock().map_err(|_| lock_error(ruby))?;
        let server = lock.as_mut().ok_or_else(|| server_stopped_error(ruby))?;

        let commit_inner = commit.inner.lock().map_err(|_| lock_error(ruby))?.clone();

        runtime()
            .map_err(|err| runtime_error(ruby, err))?
            .block_on(server.add_commit(&commit.owner, &commit.repo, commit_inner));
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
        let mut lock = rb_self.server.lock().map_err(|_| lock_error(ruby))?;
        let server = lock.as_mut().ok_or_else(|| server_stopped_error(ruby))?;

        let asset_inner = asset.inner.lock().map_err(|_| lock_error(ruby))?.clone();

        runtime()
            .map_err(|err| runtime_error(ruby, err))?
            .block_on(server.add_asset(&owner, &repo, &tag, asset_inner));
        Ok(())
    }

    fn clear_all_mock_behaviors(ruby: &Ruby, rb_self: &Self) -> Result<(), Error> {
        let mut lock = rb_self.server.lock().map_err(|_| lock_error(ruby))?;
        let server = lock.as_mut().ok_or_else(|| server_stopped_error(ruby))?;

        runtime()
            .map_err(|err| runtime_error(ruby, err))?
            .block_on(server.clear_all_mock_behaviors());
        Ok(())
    }
}

#[magnus::init]
pub fn init(ruby: &Ruby) -> Result<(), Error> {
    let module = ruby.define_module("GitHubMockAPI")?;

    let repository_class = module.define_class("Repository", ruby.class_object())?;
    repository_class.define_singleton_method("new", function!(Repository::new, 2))?;
    repository_class.define_method("description", method!(Repository::description, 1))?;
    repository_class.define_method(
        "clear_description",
        method!(Repository::clear_description, 0),
    )?;
    repository_class.define_method("private", method!(Repository::private, 1))?;
    repository_class.define_method("stargazers_count", method!(Repository::stargazers_count, 1))?;
    repository_class.define_method("default_branch", method!(Repository::default_branch, 1))?;
    repository_class.define_method("subscribers_count", method!(Repository::subscribers_count, 1))?;
    repository_class.define_method("network_count", method!(Repository::network_count, 1))?;

    let commit_class = module.define_class("Commit", ruby.class_object())?;
    commit_class.define_singleton_method("new", function!(Commit::new, 2))?;
    commit_class.define_method("sha", method!(Commit::sha, 1))?;
    commit_class.define_method("message", method!(Commit::message, 1))?;
    commit_class.define_method("author_name", method!(Commit::author_name, 1))?;
    commit_class.define_method("author_email", method!(Commit::author_email, 1))?;

    let release_class = module.define_class("Release", ruby.class_object())?;
    release_class.define_singleton_method("new", function!(Release::new, 3))?;
    release_class.define_method("name", method!(Release::name, 1))?;
    release_class.define_method("body", method!(Release::body, 1))?;
    release_class.define_method("target_commitish", method!(Release::target_commitish, 1))?;
    release_class.define_method("draft", method!(Release::draft, 1))?;
    release_class.define_method("prerelease", method!(Release::prerelease, 1))?;
    release_class.define_method("created_at", method!(Release::created_at, 1))?;

    let asset_class = module.define_class("Asset", ruby.class_object())?;
    asset_class.define_singleton_method("from_bytes", function!(Asset::from_bytes, 3))?;
    asset_class.define_singleton_method("from_file", function!(Asset::from_file, 3))?;

    let behavior_class = module.define_class("MockBehavior", ruby.class_object())?;
    behavior_class.define_singleton_method("new", function!(MockBehavior::new, 0))?;
    behavior_class.define_method("error", method!(MockBehavior::error, 1))?;

    let class = module.define_class("MockServer", ruby.class_object())?;
    class.define_singleton_method("start", function!(MockServer::start, 0))?;
    class.define_singleton_method("start_on", function!(MockServer::start_on, 2))?;
    class.define_method("uri", method!(MockServer::uri, 0))?;
    class.define_method("stop", method!(MockServer::stop, 0))?;
    class.define_method(
        "add_mock_behavior",
        method!(MockServer::add_mock_behavior, 1),
    )?;
    class.define_method("add_repository", method!(MockServer::add_repository, 1))?;
    class.define_method("add_release", method!(MockServer::add_release, 1))?;
    class.define_method("add_commit", method!(MockServer::add_commit, 1))?;
    class.define_method("add_asset", method!(MockServer::add_asset, 4))?;
    class.define_method(
        "clear_all_mock_behaviors",
        method!(MockServer::clear_all_mock_behaviors, 0),
    )?;

    // Ensure ServerStoppedError is defined
    let _ = module.define_class(
        "ServerStoppedError",
        ruby.exception_standard_error().as_r_class(),
    )?;

    Ok(())
}
