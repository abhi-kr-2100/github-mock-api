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
}

pub fn init_module(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<MockServer>()?;
    m.add_class::<MockError>()?;
    m.add_class::<MockBehavior>()?;
    m.add_class::<MockBehaviorBuilder>()?;
    Ok(())
}

#[pymodule(name = "github_mock_api")]
fn github_mock_api_module(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    init_module(m)
}
