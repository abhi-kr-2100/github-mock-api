use std::sync::Mutex;

use ::github_mock_api::{
    Asset as RustAsset, Commit as RustCommit, Error as MockApiError,
    MockBehavior as RustMockBehavior, MockError as RustMockError, MockServer as RustMockServer,
    Release as RustRelease, Repository as RustRepository,
};
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
        CommonError::DataLoad => PyErr::new::<PyRuntimeError, _>("data load error"),
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

    fn add_repository(&self, repository: &Repository) -> PyResult<()> {
        let lock = self.server.lock().map_err(|_| lock_error())?;
        if let Some(ref server) = *lock {
            runtime()
                .map_err(runtime_error)?
                .block_on(server.add_repository(repository.inner.clone()));
        }
        Ok(())
    }

    fn add_release(&self, owner: &str, repo: &str, release: &Release) -> PyResult<()> {
        let lock = self.server.lock().map_err(|_| lock_error())?;
        if let Some(ref server) = *lock {
            runtime()
                .map_err(runtime_error)?
                .block_on(server.add_release(owner, repo, release.inner.clone()));
        }
        Ok(())
    }

    fn add_commit(&self, owner: &str, repo: &str, commit: &Commit) -> PyResult<()> {
        let lock = self.server.lock().map_err(|_| lock_error())?;
        if let Some(ref server) = *lock {
            runtime()
                .map_err(runtime_error)?
                .block_on(server.add_commit(owner, repo, commit.inner.clone()));
        }
        Ok(())
    }

    fn add_asset(&self, owner: &str, repo: &str, tag: &str, asset: &Asset) -> PyResult<()> {
        let lock = self.server.lock().map_err(|_| lock_error())?;
        if let Some(ref server) = *lock {
            runtime()
                .map_err(runtime_error)?
                .block_on(server.add_asset(owner, repo, tag, asset.inner.clone()));
        }
        Ok(())
    }

    fn add_mock_behavior(&self, behavior: &MockBehavior) -> PyResult<()> {
        let lock = self.server.lock().map_err(|_| lock_error())?;
        if let Some(ref server) = *lock {
            runtime()
                .map_err(runtime_error)?
                .block_on(server.add_mock_behavior(behavior.inner.clone()))
                .map_err(mock_api_error)?;
        }
        Ok(())
    }

    fn clear_all_mock_behaviors(&self) -> PyResult<()> {
        let lock = self.server.lock().map_err(|_| lock_error())?;
        if let Some(ref server) = *lock {
            runtime()
                .map_err(runtime_error)?
                .block_on(server.clear_all_mock_behaviors());
        }
        Ok(())
    }
}

#[pyclass(module = "github_mock_api")]
#[derive(Clone)]
struct Repository {
    inner: RustRepository,
}

#[pymethods]
impl Repository {
    #[new]
    fn new(owner: &str, name: &str) -> Self {
        Self {
            inner: RustRepository::new(owner, name),
        }
    }

    #[staticmethod]
    fn load_from_file(path: &str) -> PyResult<Vec<Self>> {
        let repos = RustRepository::load_from_file(path)
            .map_err(|e| PyErr::new::<PyRuntimeError, _>(e.to_string()))?;
        Ok(repos.into_iter().map(|r| Self { inner: r }).collect())
    }

    #[getter]
    fn name(&self) -> String {
        self.inner.name.clone()
    }

    #[setter]
    fn set_name(&mut self, name: String) {
        self.inner.name = name;
    }

    // Add other fields as needed, or use a generic approach if possible.
    // For now, I'll provide the builder methods.
    fn description(&mut self, description: String) -> Self {
        self.inner = self.inner.clone().description(description);
        self.clone()
    }

    fn private(&mut self, private: bool) -> Self {
        self.inner = self.inner.clone().private(private);
        self.clone()
    }

    fn stargazers_count(&mut self, count: u64) -> Self {
        self.inner = self.inner.clone().stargazers_count(count);
        self.clone()
    }

    fn default_branch(&mut self, branch: String) -> Self {
        self.inner = self.inner.clone().default_branch(branch);
        self.clone()
    }
}

#[pyclass(module = "github_mock_api")]
#[derive(Clone)]
struct Release {
    inner: RustRelease,
}

#[pymethods]
impl Release {
    #[new]
    fn new(owner: &str, repo: &str, tag_name: &str) -> Self {
        Self {
            inner: RustRelease::new(owner, repo, tag_name),
        }
    }

    #[staticmethod]
    fn load_from_file(path: &str, owner: &str, repo: &str) -> PyResult<Vec<Self>> {
        let releases = RustRelease::load_from_file(path, owner, repo)
            .map_err(|e| PyErr::new::<PyRuntimeError, _>(e.to_string()))?;
        Ok(releases.into_iter().map(|r| Self { inner: r }).collect())
    }

    fn name(&mut self, name: String) -> Self {
        self.inner = self.inner.clone().name(name);
        self.clone()
    }

    fn body(&mut self, body: String) -> Self {
        self.inner = self.inner.clone().body(body);
        self.clone()
    }

    fn draft(&mut self, draft: bool) -> Self {
        self.inner = self.inner.clone().draft(draft);
        self.clone()
    }

    fn prerelease(&mut self, prerelease: bool) -> Self {
        self.inner = self.inner.clone().prerelease(prerelease);
        self.clone()
    }
}

#[pyclass(module = "github_mock_api")]
#[derive(Clone)]
struct Commit {
    inner: RustCommit,
}

#[pymethods]
impl Commit {
    #[new]
    fn new(owner: &str, repo: &str) -> Self {
        Self {
            inner: RustCommit::new(owner, repo),
        }
    }

    #[staticmethod]
    fn load_from_file(path: &str, owner: &str, repo: &str) -> PyResult<Vec<Self>> {
        let commits = RustCommit::load_from_file(path, owner, repo)
            .map_err(|e| PyErr::new::<PyRuntimeError, _>(e.to_string()))?;
        Ok(commits.into_iter().map(|c| Self { inner: c }).collect())
    }

    fn sha(&mut self, sha: String) -> Self {
        self.inner = self.inner.clone().sha(sha);
        self.clone()
    }

    fn message(&mut self, message: String) -> Self {
        self.inner = self.inner.clone().message(message);
        self.clone()
    }
}

#[pyclass(module = "github_mock_api")]
#[derive(Clone)]
struct Asset {
    inner: RustAsset,
}

#[pymethods]
impl Asset {
    #[staticmethod]
    fn from_bytes(name: String, bytes: Vec<u8>, content_type: String) -> Self {
        Self {
            inner: RustAsset::from_bytes(name, bytes, content_type),
        }
    }

    #[staticmethod]
    fn from_path(name: String, path: String, content_type: String) -> Self {
        Self {
            inner: RustAsset::from_path(name, path, content_type),
        }
    }
}

#[pyclass(module = "github_mock_api")]
#[derive(Clone, Copy, PartialEq, Eq)]
enum MockError {
    InternalServerError,
    RateLimitExceeded,
}

#[pyclass(module = "github_mock_api")]
#[derive(Clone)]
struct MockBehavior {
    inner: RustMockBehavior,
}

#[pymethods]
impl MockBehavior {
    #[staticmethod]
    fn error(error: MockError) -> Self {
        let rust_error = match error {
            MockError::InternalServerError => RustMockError::InternalServerError,
            MockError::RateLimitExceeded => RustMockError::RateLimitExceeded,
        };
        Self {
            inner: RustMockBehavior::builder().error(rust_error).build(),
        }
    }
}

pub fn init_module(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<MockServer>()?;
    m.add_class::<Repository>()?;
    m.add_class::<Release>()?;
    m.add_class::<Commit>()?;
    m.add_class::<Asset>()?;
    m.add_class::<MockError>()?;
    m.add_class::<MockBehavior>()?;
    Ok(())
}

#[pymodule(name = "github_mock_api")]
fn github_mock_api_module(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    init_module(m)
}
