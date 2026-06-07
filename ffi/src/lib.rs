#[diplomat::bridge]
mod ffi {
    use std::fmt::Write as _;

    use diplomat_runtime::DiplomatWrite;
    use github_mock_api::{
        Asset as RustAsset, Commit as RustCommit, Error, MockBehavior as RustMockBehavior,
        MockError as RustMockError, MockServer as RustMockServer, Release as RustRelease,
        Repository as RustRepository,
    };

    use github_mock_api_ffi_common::{CommonError, parse_host, runtime};

    #[diplomat::rust_link(github_mock_api::MockServer, Struct)]
    #[diplomat::opaque_mut]
    pub struct MockServer {
        server: RustMockServer,
    }

    #[diplomat::rust_link(github_mock_api::Repository, Struct)]
    #[diplomat::opaque_mut]
    pub struct Repository {
        pub(crate) inner: RustRepository,
    }

    #[diplomat::rust_link(github_mock_api::Release, Struct)]
    #[diplomat::opaque_mut]
    pub struct Release {
        pub(crate) inner: RustRelease,
    }

    #[diplomat::rust_link(github_mock_api::Commit, Struct)]
    #[diplomat::opaque_mut]
    pub struct Commit {
        pub(crate) inner: RustCommit,
    }

    #[diplomat::rust_link(github_mock_api::Asset, Struct)]
    #[diplomat::opaque_mut]
    pub struct Asset {
        pub(crate) inner: RustAsset,
    }

    #[diplomat::rust_link(github_mock_api::MockBehavior, Struct)]
    #[diplomat::opaque_mut]
    pub struct MockBehavior {
        pub(crate) inner: RustMockBehavior,
    }

    #[diplomat::rust_link(github_mock_api::MockError, Enum)]
    pub enum MockError {
        InternalServerError,
        RateLimitExceeded,
    }

    #[diplomat::attr(supports = custom_errors, error)]
    pub enum MockServerError {
        Io,
        Shutdown,
        Join,
        InvalidHost,
        Conflict,
        DataLoad,
    }

    impl From<Error> for MockServerError {
        fn from(err: Error) -> Self {
            CommonError::from(err).into()
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
                CommonError::DataLoad => MockServerError::DataLoad,
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

        pub fn add_repository(&mut self, repository: &Repository) {
            runtime()
                .expect("runtime")
                .block_on(self.server.add_repository(repository.inner.clone()));
        }

        pub fn add_repositories_from_file(&mut self, path: &str) -> Result<(), MockServerError> {
            let repos = RustRepository::load_from_file(path)
                .map_err(|e| MockServerError::from(CommonError::from(e)))?;
            for repo in repos {
                runtime()
                    .expect("runtime")
                    .block_on(self.server.add_repository(repo));
            }
            Ok(())
        }

        pub fn add_release(&mut self, owner: &str, repo: &str, release: &Release) {
            runtime()
                .expect("runtime")
                .block_on(self.server.add_release(owner, repo, release.inner.clone()));
        }

        pub fn add_releases_from_file(
            &mut self,
            path: &str,
            owner: &str,
            repo: &str,
        ) -> Result<(), MockServerError> {
            let releases = RustRelease::load_from_file(path, owner, repo)
                .map_err(|e| MockServerError::from(CommonError::from(e)))?;
            for release in releases {
                runtime()
                    .expect("runtime")
                    .block_on(self.server.add_release(owner, repo, release));
            }
            Ok(())
        }

        pub fn add_commit(&mut self, owner: &str, repo: &str, commit: &Commit) {
            runtime()
                .expect("runtime")
                .block_on(self.server.add_commit(owner, repo, commit.inner.clone()));
        }

        pub fn add_commits_from_file(
            &mut self,
            path: &str,
            owner: &str,
            repo: &str,
        ) -> Result<(), MockServerError> {
            let commits = RustCommit::load_from_file(path, owner, repo)
                .map_err(|e| MockServerError::from(CommonError::from(e)))?;
            for commit in commits {
                runtime()
                    .expect("runtime")
                    .block_on(self.server.add_commit(owner, repo, commit));
            }
            Ok(())
        }

        pub fn add_asset(&mut self, owner: &str, repo: &str, tag: &str, asset: &Asset) {
            runtime()
                .expect("runtime")
                .block_on(self.server.add_asset(owner, repo, tag, asset.inner.clone()));
        }

        pub fn add_mock_behavior(
            &mut self,
            behavior: &MockBehavior,
        ) -> Result<(), MockServerError> {
            runtime()?
                .block_on(self.server.add_mock_behavior(behavior.inner.clone()))
                .map_err(MockServerError::from)
        }

        pub fn clear_all_mock_behaviors(&mut self) {
            runtime()
                .expect("runtime")
                .block_on(self.server.clear_all_mock_behaviors());
        }
    }

    impl Repository {
        pub fn new(owner: &str, name: &str) -> Box<Repository> {
            Box::new(Repository {
                inner: RustRepository::new(owner, name),
            })
        }
    }

    impl Release {
        pub fn new(owner: &str, repo: &str, tag_name: &str) -> Box<Release> {
            Box::new(Release {
                inner: RustRelease::new(owner, repo, tag_name),
            })
        }
    }

    impl Commit {
        pub fn new(owner: &str, repo: &str) -> Box<Commit> {
            Box::new(Commit {
                inner: RustCommit::new(owner, repo),
            })
        }
    }

    impl Asset {
        pub fn from_bytes(name: &str, bytes: &[u8], content_type: &str) -> Box<Asset> {
            Box::new(Asset {
                inner: RustAsset::from_bytes(name, bytes.to_vec(), content_type),
            })
        }

        pub fn from_path(name: &str, path: &str, content_type: &str) -> Box<Asset> {
            Box::new(Asset {
                inner: RustAsset::from_path(name, path, content_type),
            })
        }
    }

    impl MockBehavior {
        pub fn new_error(error: MockError) -> Box<MockBehavior> {
            let rust_error = match error {
                MockError::InternalServerError => RustMockError::InternalServerError,
                MockError::RateLimitExceeded => RustMockError::RateLimitExceeded,
            };
            Box::new(MockBehavior {
                inner: RustMockBehavior::builder().error(rust_error).build(),
            })
        }
    }
}
