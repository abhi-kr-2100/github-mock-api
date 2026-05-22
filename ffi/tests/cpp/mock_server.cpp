#include <cassert>
#include <iostream>
#include <memory>
#include <optional>
#include <string>

#include "github_mock_api/MockServer.hpp"

int main() {
    auto started = github_mock_api_ffi::MockServer::start();
    if (started.is_err()) {
        std::cerr << "MockServer::start failed\n";
        return 1;
    }

    std::optional<std::unique_ptr<github_mock_api_ffi::MockServer>> maybe_server = std::move(started).ok();
    if (!maybe_server.has_value()) {
        std::cerr << "MockServer::start returned no server\n";
        return 1;
    }

    std::unique_ptr<github_mock_api_ffi::MockServer> server = std::move(maybe_server.value());
    const std::string uri = server->uri();
    if (uri.rfind("http://127.0.0.1:", 0) != 0) {
        std::cerr << "unexpected uri: " << uri << "\n";
        return 1;
    }

    auto stopped = server->stop();
    if (stopped.is_err()) {
        std::cerr << "MockServer::stop failed\n";
        return 1;
    }

    return 0;
}
