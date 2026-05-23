use std::net::IpAddr;
use std::sync::{Mutex, OnceLock};

use ::github_mock_api::{Error as MockApiError, MockServer as RustMockServer};
use pyo3::{exceptions::PyValueError, prelude::*, PyErr};
use tokio::runtime::Runtime;

#[derive(Debug, Clone, Copy)]
enum RuntimeError {
    Io,
}

fn runtime() -> Result<&'static Runtime, RuntimeError> {
    static RUNTIME: OnceLock<Result<Runtime, RuntimeError>> = OnceLock::new();
    RUNTIME
        .get_or_init(|| Runtime::new().map_err(|_| RuntimeError::Io))
        .as_ref()
        .map_err(|err| *err)
}

fn parse_host(host: &str) -> Result<IpAddr, PyErr> {
    host.parse::<IpAddr>()
        .map_err(|_| PyErr::new::<PyValueError, _>("invalid host"))
}

fn runtime_error(err: RuntimeError) -> PyErr {
    match err {
        RuntimeError::Io => {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>("failed to initialize runtime")
        }
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
        let host = parse_host(host)?;
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
}

pub fn init_module(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<MockServer>()?;
    Ok(())
}

#[pymodule(name = "github_mock_api")]
fn github_mock_api_module(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    init_module(m)
}
