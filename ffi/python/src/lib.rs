use std::sync::Mutex;

use ::github_mock_api::{Error as MockApiError, MockServer as RustMockServer};
use github_mock_api_ffi_common::{CommonError, parse_host, runtime};
use pyo3::{
    PyErr,
    exceptions::{PyRuntimeError, PyValueError},
    prelude::*,
};

fn runtime_error(err: CommonError) -> PyErr {
    match err {
        CommonError::Io => PyErr::new::<PyRuntimeError, _>("failed to initialize runtime"),
        CommonError::InvalidHost => PyErr::new::<PyValueError, _>("invalid host"),
        CommonError::Shutdown => PyErr::new::<PyRuntimeError, _>("shutdown error"),
        CommonError::Join => PyErr::new::<PyRuntimeError, _>("join error"),
        CommonError::Conflict => PyErr::new::<PyRuntimeError, _>("mock behavior conflict"),
    }
}

fn mock_api_error(err: MockApiError) -> PyErr {
    match err {
        MockApiError::Io(err) => PyErr::new::<pyo3::exceptions::PyIOError, _>(err.to_string()),
        MockApiError::ShutdownError(err) => {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(err.to_string())
        }
        MockApiError::JoinError(err) => {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(err.to_string())
        }
        MockApiError::Conflict(err) => PyErr::new::<PyRuntimeError, _>(err),
    }
}

fn lock_error() -> PyErr {
    PyErr::new::<pyo3::exceptions::PyRuntimeError, _>("mock server lock poisoned")
}

fn load_error(err: ::github_mock_api::LoadError) -> PyErr {
    match err {
        ::github_mock_api::LoadError::Io { .. } => {
            PyErr::new::<pyo3::exceptions::PyIOError, _>(err.to_string())
        }
        ::github_mock_api::LoadError::Json { .. } => {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(err.to_string())
        }
    }
}

#[pyclass(
    module = "github_mock_api",
    rename_all = "SCREAMING_SNAKE_CASE",
    from_py_object
)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum MockError {
    InternalServerError,
    RateLimitExceeded,
}

impl From<MockError> for ::github_mock_api::MockError {
    fn from(err: MockError) -> Self {
        match err {
            MockError::InternalServerError => ::github_mock_api::MockError::InternalServerError,
            MockError::RateLimitExceeded => ::github_mock_api::MockError::RateLimitExceeded,
        }
    }
}

#[pyclass(module = "github_mock_api", from_py_object)]
#[derive(Clone)]
pub struct MockBehavior {
    inner: ::github_mock_api::MockBehavior,
}

#[pymethods]
impl MockBehavior {
    #[staticmethod]
    fn builder() -> MockBehaviorBuilder {
        MockBehaviorBuilder { error: None }
    }
}

#[pyclass(module = "github_mock_api", from_py_object)]
#[derive(Clone)]
pub struct MockBehaviorBuilder {
    error: Option<MockError>,
}

#[pymethods]
impl MockBehaviorBuilder {
    pub fn error(&self, error: MockError) -> Self {
        let mut new = self.clone();
        new.error = Some(error);
        new
    }

    pub fn build(&self) -> MockBehavior {
        let mut builder = ::github_mock_api::MockBehavior::builder();
        if let Some(err) = self.error {
            builder = builder.error(err.into());
        }
        MockBehavior {
            inner: builder.build(),
        }
    }
}

#[pyclass(module = "github_mock_api", from_py_object)]
#[derive(Clone)]
pub struct Repository {
    inner: ::github_mock_api::Repository,
}

#[pymethods]
impl Repository {
    #[staticmethod]
    pub fn builder(owner: String, name: String) -> RepositoryBuilder {
        RepositoryBuilder::new(owner, name)
    }

    #[staticmethod]
    pub fn load_from_file(path: String) -> PyResult<Vec<Self>> {
        let repos = ::github_mock_api::Repository::load_from_file(path).map_err(load_error)?;
        Ok(repos.into_iter().map(|inner| Self { inner }).collect())
    }
}

#[pyclass(module = "github_mock_api", from_py_object)]
#[derive(Clone)]
pub struct RepositoryBuilder {
    inner: ::github_mock_api::Repository,
}

#[pymethods]
impl RepositoryBuilder {
    #[staticmethod]
    pub fn new(owner: String, name: String) -> Self {
        Self {
            inner: ::github_mock_api::Repository::new(&owner, &name),
        }
    }

    pub fn description(&self, description: String) -> Self {
        let mut new = self.clone();
        new.inner = new.inner.description(description);
        new
    }

    pub fn clear_description(&self) -> Self {
        let mut new = self.clone();
        new.inner = new.inner.clear_description();
        new
    }

    pub fn private(&self, private: bool) -> Self {
        let mut new = self.clone();
        new.inner = new.inner.private(private);
        new
    }

    pub fn stargazers_count(&self, count: u64) -> Self {
        let mut new = self.clone();
        new.inner = new.inner.stargazers_count(count);
        new
    }

    pub fn default_branch(&self, branch: String) -> Self {
        let mut new = self.clone();
        new.inner = new.inner.default_branch(branch);
        new
    }

    pub fn subscribers_count(&self, count: u64) -> Self {
        let mut new = self.clone();
        new.inner = new.inner.subscribers_count(count);
        new
    }

    pub fn network_count(&self, count: u64) -> Self {
        let mut new = self.clone();
        new.inner = new.inner.network_count(count);
        new
    }

    pub fn build(&self) -> Repository {
        Repository {
            inner: self.inner.clone(),
        }
    }
}

#[pyclass(module = "github_mock_api", from_py_object)]
#[derive(Clone)]
pub struct Release {
    inner: ::github_mock_api::Release,
}

#[pymethods]
impl Release {
    #[staticmethod]
    pub fn builder(owner: String, repo: String, tag_name: String) -> ReleaseBuilder {
        ReleaseBuilder::new(owner, repo, tag_name)
    }

    #[staticmethod]
    pub fn load_from_file(path: String, owner: String, repo: String) -> PyResult<Vec<Self>> {
        let releases =
            ::github_mock_api::Release::load_from_file(path, &owner, &repo).map_err(load_error)?;
        Ok(releases.into_iter().map(|inner| Self { inner }).collect())
    }
}

#[pyclass(module = "github_mock_api", from_py_object)]
#[derive(Clone)]
pub struct ReleaseBuilder {
    inner: ::github_mock_api::Release,
}

#[pymethods]
impl ReleaseBuilder {
    #[staticmethod]
    pub fn new(owner: String, repo: String, tag_name: String) -> Self {
        Self {
            inner: ::github_mock_api::Release::new(&owner, &repo, &tag_name),
        }
    }

    pub fn name(&self, name: String) -> Self {
        let mut new = self.clone();
        new.inner = new.inner.name(name);
        new
    }

    pub fn body(&self, body: String) -> Self {
        let mut new = self.clone();
        new.inner = new.inner.body(body);
        new
    }

    pub fn target_commitish(&self, commitish: String) -> Self {
        let mut new = self.clone();
        new.inner = new.inner.target_commitish(commitish);
        new
    }

    pub fn draft(&self, draft: bool) -> Self {
        let mut new = self.clone();
        new.inner = new.inner.draft(draft);
        new
    }

    pub fn prerelease(&self, prerelease: bool) -> Self {
        let mut new = self.clone();
        new.inner = new.inner.prerelease(prerelease);
        new
    }

    pub fn created_at(&self, created_at: String) -> Self {
        let mut new = self.clone();
        new.inner = new.inner.created_at(created_at);
        new
    }

    pub fn build(&self) -> Release {
        Release {
            inner: self.inner.clone(),
        }
    }
}

#[pyclass(module = "github_mock_api", from_py_object)]
#[derive(Clone)]
pub struct Commit {
    inner: ::github_mock_api::Commit,
}

#[pymethods]
impl Commit {
    #[staticmethod]
    pub fn builder(owner: String, repo: String) -> CommitBuilder {
        CommitBuilder::new(owner, repo)
    }

    #[staticmethod]
    pub fn load_from_file(path: String, owner: String, repo: String) -> PyResult<Vec<Self>> {
        let commits =
            ::github_mock_api::Commit::load_from_file(path, &owner, &repo).map_err(load_error)?;
        Ok(commits.into_iter().map(|inner| Self { inner }).collect())
    }
}

#[pyclass(module = "github_mock_api", from_py_object)]
#[derive(Clone)]
pub struct CommitBuilder {
    inner: ::github_mock_api::Commit,
}

#[pymethods]
impl CommitBuilder {
    #[staticmethod]
    pub fn new(owner: String, repo: String) -> Self {
        Self {
            inner: ::github_mock_api::Commit::new(&owner, &repo),
        }
    }

    pub fn sha(&self, sha: String) -> Self {
        let mut new = self.clone();
        new.inner = new.inner.sha(sha);
        new
    }

    pub fn message(&self, message: String) -> Self {
        let mut new = self.clone();
        new.inner = new.inner.message(message);
        new
    }

    pub fn author_name(&self, name: String) -> Self {
        let mut new = self.clone();
        new.inner = new.inner.author_name(name);
        new
    }

    pub fn author_email(&self, email: String) -> Self {
        let mut new = self.clone();
        new.inner = new.inner.author_email(email);
        new
    }

    pub fn build(&self) -> Commit {
        Commit {
            inner: self.inner.clone(),
        }
    }
}

#[pyclass(module = "github_mock_api", from_py_object)]
#[derive(Clone)]
pub struct Asset {
    inner: ::github_mock_api::Asset,
}

#[pymethods]
impl Asset {
    #[staticmethod]
    pub fn from_bytes(name: String, bytes: Vec<u8>, content_type: String) -> Self {
        Self {
            inner: ::github_mock_api::Asset::from_bytes(name, bytes, content_type),
        }
    }

    #[staticmethod]
    pub fn from_file(name: String, path: String, content_type: String) -> Self {
        Self {
            inner: ::github_mock_api::Asset::from_path(
                name,
                std::path::PathBuf::from(path),
                content_type,
            ),
        }
    }
}

#[pyclass(module = "github_mock_api")]
struct MockServer {
    server: Mutex<Option<RustMockServer>>,
    uri: String,
}

#[pymethods]
impl MockServer {
    #[staticmethod]
    fn start() -> PyResult<Self> {
        let server = runtime()
            .map_err(runtime_error)?
            .block_on(RustMockServer::start())
            .map_err(mock_api_error)?;
        let uri = server.uri();
        Ok(Self {
            server: Mutex::new(Some(server)),
            uri,
        })
    }

    #[staticmethod]
    fn start_on(host: &str, port: u16) -> PyResult<Self> {
        let host = parse_host(host).map_err(runtime_error)?;
        let server = runtime()
            .map_err(runtime_error)?
            .block_on(RustMockServer::start_on(host, port))
            .map_err(mock_api_error)?;
        let uri = server.uri();
        Ok(Self {
            server: Mutex::new(Some(server)),
            uri,
        })
    }

    fn uri(&self) -> PyResult<String> {
        Ok(self.uri.clone())
    }

    fn stop(&self) -> PyResult<()> {
        if let Some(mut server) = self.server.lock().map_err(|_| lock_error())?.take() {
            runtime()
                .map_err(runtime_error)?
                .block_on(server.stop())
                .map_err(mock_api_error)?;
        }
        Ok(())
    }

    fn add_mock_behavior(&self, behavior: MockBehavior) -> PyResult<()> {
        let lock = self.server.lock().map_err(|_| lock_error())?;
        let server = lock
            .as_ref()
            .ok_or_else(|| PyErr::new::<PyRuntimeError, _>("server is stopped"))?;
        runtime()
            .map_err(runtime_error)?
            .block_on(server.add_mock_behavior(behavior.inner))
            .map_err(mock_api_error)?;
        Ok(())
    }

    fn clear_all_mock_behaviors(&self) -> PyResult<()> {
        let lock = self.server.lock().map_err(|_| lock_error())?;
        let server = lock
            .as_ref()
            .ok_or_else(|| PyErr::new::<PyRuntimeError, _>("server is stopped"))?;
        runtime()
            .map_err(runtime_error)?
            .block_on(server.clear_all_mock_behaviors());
        Ok(())
    }

    fn add_repository(&self, repository: Repository) -> PyResult<()> {
        let lock = self.server.lock().map_err(|_| lock_error())?;
        let server = lock
            .as_ref()
            .ok_or_else(|| PyErr::new::<PyRuntimeError, _>("server is stopped"))?;
        runtime()
            .map_err(runtime_error)?
            .block_on(server.add_repository(repository.inner));
        Ok(())
    }

    fn add_release(&self, owner: String, repo: String, release: Release) -> PyResult<()> {
        let lock = self.server.lock().map_err(|_| lock_error())?;
        let server = lock
            .as_ref()
            .ok_or_else(|| PyErr::new::<PyRuntimeError, _>("server is stopped"))?;
        runtime()
            .map_err(runtime_error)?
            .block_on(server.add_release(&owner, &repo, release.inner));
        Ok(())
    }

    fn add_commit(&self, owner: String, repo: String, commit: Commit) -> PyResult<()> {
        let lock = self.server.lock().map_err(|_| lock_error())?;
        let server = lock
            .as_ref()
            .ok_or_else(|| PyErr::new::<PyRuntimeError, _>("server is stopped"))?;
        runtime()
            .map_err(runtime_error)?
            .block_on(server.add_commit(&owner, &repo, commit.inner));
        Ok(())
    }

    fn add_asset(&self, owner: String, repo: String, tag: String, asset: Asset) -> PyResult<()> {
        let lock = self.server.lock().map_err(|_| lock_error())?;
        let server = lock
            .as_ref()
            .ok_or_else(|| PyErr::new::<PyRuntimeError, _>("server is stopped"))?;
        runtime().map_err(runtime_error)?.block_on(server.add_asset(
            &owner,
            &repo,
            &tag,
            asset.inner,
        ));
        Ok(())
    }
}

pub fn init_module(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<MockServer>()?;
    m.add_class::<MockError>()?;
    m.add_class::<MockBehavior>()?;
    m.add_class::<MockBehaviorBuilder>()?;
    m.add_class::<Repository>()?;
    m.add_class::<RepositoryBuilder>()?;
    m.add_class::<Release>()?;
    m.add_class::<ReleaseBuilder>()?;
    m.add_class::<Commit>()?;
    m.add_class::<CommitBuilder>()?;
    m.add_class::<Asset>()?;
    Ok(())
}

#[pymodule(name = "github_mock_api")]
fn github_mock_api_module(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    init_module(m)
}
