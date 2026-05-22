use std::net::IpAddr;
use std::sync::{Mutex, OnceLock};

use github_mock_api::{Error as MockApiError, MockServer as RustMockServer};
use magnus::{Error, Ruby, function, method, prelude::*, wrap};
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

fn parse_host(ruby: &Ruby, host: &str) -> Result<IpAddr, Error> {
    host.parse::<IpAddr>()
        .map_err(|_| Error::new(ruby.exception_arg_error(), "invalid host"))
}

fn runtime_error(ruby: &Ruby, err: RuntimeError) -> Error {
    match err {
        RuntimeError::Io => Error::new(ruby.exception_io_error(), "failed to initialize runtime"),
    }
}

fn mock_api_error(ruby: &Ruby, err: MockApiError) -> Error {
    match err {
        MockApiError::Io(err) => Error::new(ruby.exception_io_error(), err.to_string()),
        MockApiError::ShutdownError(err) => {
            Error::new(ruby.exception_runtime_error(), err.to_string())
        }
        MockApiError::JoinError(err) => Error::new(ruby.exception_runtime_error(), err.to_string()),
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
        let host = parse_host(ruby, &host)?;
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
}

#[magnus::init]
pub fn init(ruby: &Ruby) -> Result<(), Error> {
    let module = ruby.define_module("GitHubMockAPI")?;
    let class = module.define_class("MockServer", ruby.class_object())?;
    class.define_singleton_method("start", function!(MockServer::start, 0))?;
    class.define_singleton_method("start_on", function!(MockServer::start_on, 2))?;
    class.define_method("uri", method!(MockServer::uri, 0))?;
    class.define_method("stop", method!(MockServer::stop, 0))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn start_uri_and_stop_from_rust_wrapper() -> Result<(), Error> {
        let ruby = unsafe { magnus::embed::init() };
        let server = MockServer::start(&ruby)?;

        let uri = MockServer::uri(&ruby, &server)?;
        assert!(uri.starts_with("http://127.0.0.1:"));
        MockServer::stop(&ruby, &server)?;
        MockServer::stop(&ruby, &server)?;

        // uri should still be available after stop
        assert_eq!(MockServer::uri(&ruby, &server)?, uri);

        Ok(())
    }
}
