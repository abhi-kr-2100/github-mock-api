use std::sync::Mutex;

use github_mock_api::{
    Error as MockApiError, MockBehavior as RustMockBehavior, MockError,
    MockServer as RustMockServer, Repository as RustRepository,
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
