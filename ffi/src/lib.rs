#[diplomat::bridge]
mod ffi {
    use std::fmt::Write as _;

    use diplomat_runtime::DiplomatWrite;
    use github_mock_api::{Error, MockServer as RustMockServer, Repository as RustRepository};

    use github_mock_api_ffi_common::{CommonError, parse_host, runtime};

    #[diplomat::rust_link(github_mock_api::MockError, Enum)]
    pub enum MockError {
        InternalServerError,
        RateLimitExceeded,
    }

    impl From<MockError> for github_mock_api::MockError {
        fn from(error: MockError) -> Self {
            match error {
                MockError::InternalServerError => github_mock_api::MockError::InternalServerError,
                MockError::RateLimitExceeded => github_mock_api::MockError::RateLimitExceeded,
            }
        }
    }

    #[diplomat::rust_link(github_mock_api::MockBehavior, Struct)]
    #[diplomat::opaque]
    #[derive(Clone, Copy)]
    pub struct MockBehavior {
        pub(crate) error: Option<MockError>,
    }

    impl MockBehavior {
        /// Create a new mock behavior builder.
        pub fn new() -> Box<MockBehavior> {
            Box::new(MockBehavior { error: None })
        }

        /// Set the error for the mock behavior.
        pub fn with_error(&self, error: MockError) -> Box<MockBehavior> {
            let mut new = *self;
            new.error = Some(error);
            Box::new(new)
        }

        pub(crate) fn build(&self) -> github_mock_api::MockBehavior {
            let mut builder = github_mock_api::MockBehavior::builder();
            if let Some(error) = self.error {
                builder = builder.error(error.into());
            }
            builder.build()
        }
    }

    #[diplomat::rust_link(github_mock_api::Repository, Struct)]
    #[diplomat::opaque]
    #[derive(Clone)]
    pub struct Repository {
        pub(crate) inner: RustRepository,
    }

    impl Repository {
        /// Create a new repository builder.
        pub fn new(owner: &str, name: &str) -> Box<Repository> {
            Box::new(Repository {
                inner: RustRepository::new(owner, name),
            })
        }

        /// Set the description for the repository.
        pub fn with_description(&self, description: &str) -> Box<Repository> {
            let mut new = self.clone();
            new.inner = new.inner.description(description);
            Box::new(new)
        }

        /// Set whether the repository is private.
        pub fn with_private(&self, private: bool) -> Box<Repository> {
            let mut new = self.clone();
            new.inner = new.inner.private(private);
            Box::new(new)
        }

        /// Set the stargazers count for the repository.
        pub fn with_stargazers_count(&self, count: u64) -> Box<Repository> {
            let mut new = self.clone();
            new.inner = new.inner.stargazers_count(count);
            Box::new(new)
        }

        /// Set the default branch for the repository.
        pub fn with_default_branch(&self, branch: &str) -> Box<Repository> {
            let mut new = self.clone();
            new.inner = new.inner.default_branch(branch);
            Box::new(new)
        }

        pub(crate) fn build(&self) -> RustRepository {
            self.inner.clone()
        }
    }

    #[diplomat::rust_link(github_mock_api::MockServer, Struct)]
    #[diplomat::opaque_mut]
    pub struct MockServer {
        server: RustMockServer,
    }

    #[diplomat::attr(supports = custom_errors, error)]
    pub enum MockServerError {
        Io,
        Shutdown,
        Join,
        InvalidHost,
        Conflict,
    }

    impl From<Error> for MockServerError {
        fn from(err: Error) -> Self {
            match CommonError::from(err) {
                CommonError::Io => MockServerError::Io,
                CommonError::Shutdown => MockServerError::Shutdown,
                CommonError::Join => MockServerError::Join,
                CommonError::InvalidHost => MockServerError::InvalidHost,
                CommonError::Conflict => MockServerError::Conflict,
            }
        }
    }

    impl From<CommonError> for MockServerError {
        fn from(err: CommonError) -> Self {
            match err {
                CommonError::Io => MockServerError::Io,
                CommonError::Shutdown => MockServerError::Shutdown,
                CommonError::Join => MockServerError::Join,
                CommonError::InvalidHost => MockServerError::InvalidHost,
                CommonError::Conflict => MockServerError::Conflict,
            }
        }
    }

    impl MockServer {
        /// Start a mock server on `127.0.0.1` with a random available port.
        pub fn start() -> Result<Box<MockServer>, MockServerError> {
            let server = runtime()?
                .block_on(RustMockServer::start())
                .map_err(MockServerError::from)?;
            Ok(Box::new(MockServer { server }))
        }

        /// Start a mock server on the given host and port (`0` picks a random port).
        pub fn start_on(host: &str, port: u16) -> Result<Box<MockServer>, MockServerError> {
            let host = parse_host(host)?;
            let server = runtime()?
                .block_on(RustMockServer::start_on(host, port))
                .map_err(MockServerError::from)?;
            Ok(Box::new(MockServer { server }))
        }

        /// Write the server base URI (for example `http://127.0.0.1:3000`) into `write`.
        pub fn uri(&self, write: &mut DiplomatWrite) {
            let _ = write.write_str(&self.server.uri());
            write.flush();
        }

        /// Stop the server and wait for background tasks to finish.
        pub fn stop(&mut self) -> Result<(), MockServerError> {
            runtime()?
                .block_on(self.server.stop())
                .map_err(MockServerError::from)
        }

        /// Add a mock behavior to the server.
        pub fn add_mock_behavior(&self, behavior: &MockBehavior) -> Result<(), MockServerError> {
            runtime()?
                .block_on(self.server.add_mock_behavior(behavior.build()))
                .map_err(MockServerError::from)
        }

        /// Clear all mock behaviors from the server.
        pub fn clear_all_mock_behaviors(&self) -> Result<(), MockServerError> {
            runtime()?.block_on(self.server.clear_all_mock_behaviors());
            Ok(())
        }

        /// Add a repository to the server.
        pub fn add_repository(&self, repository: &Repository) -> Result<(), MockServerError> {
            // Note: Rust add_repository is currently infallible, but we return a Result
            // to match other registration methods and for future compatibility.
            runtime()?.block_on(self.server.add_repository(repository.build()));
            Ok(())
        }
    }
}
