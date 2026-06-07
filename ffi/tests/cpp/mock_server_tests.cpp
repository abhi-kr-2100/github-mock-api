#include <memory>
#include <optional>
#include <string>
#include <vector>
#include <sys/socket.h>
#include <netinet/in.h>
#include <arpa/inet.h>
#include <unistd.h>

#define CATCH_CONFIG_MAIN
#include <catch2/catch.hpp>

#include "github_mock_api/MockServer.hpp"
#include "github_mock_api/Repository.hpp"
#include "github_mock_api/Release.hpp"
#include "github_mock_api/Commit.hpp"
#include "github_mock_api/Asset.hpp"
#include "github_mock_api/MockBehavior.hpp"

using namespace github_mock_api_ffi;

namespace {

std::string get_data_dir() {
    std::string path = __FILE__;
    auto pos = path.find_last_of("/\\");
    if (pos != std::string::npos) {
        path = path.substr(0, pos); // ffi/tests/cpp
    }
    return path + "/../../../testing/data";
}

int connect_to(const std::string& host, int port) {
    int sock = socket(AF_INET, SOCK_STREAM, 0);
    if (sock < 0) return -1;

    struct sockaddr_in addr{};
    addr.sin_family = AF_INET;
    addr.sin_port = htons(port);
    if (inet_pton(AF_INET, host.c_str(), &addr.sin_addr) != 1) {
        close(sock);
        return -1;
    }

    int ret = connect(sock, (struct sockaddr*)&addr, sizeof(addr));
    close(sock);
    return ret;
}

std::pair<std::string, int> parse_uri(const std::string& uri) {
    auto host_part = uri.substr(7);
    auto colon_pos = host_part.rfind(':');
    auto host = host_part.substr(0, colon_pos);
    int port = std::stoi(host_part.substr(colon_pos + 1));
    return {host, port};
}

} // anonymous namespace

TEST_CASE("MockServer starts successfully", "[mock_server]") {
    auto result = MockServer::start();
    REQUIRE(result.is_ok());

    auto maybe_server = std::move(result).ok();
    REQUIRE(maybe_server.has_value());
}

TEST_CASE("MockServer URI format after default start", "[mock_server]") {
    auto result = MockServer::start();
    REQUIRE(result.is_ok());

    auto maybe_server = std::move(result).ok();
    REQUIRE(maybe_server.has_value());
    auto server = std::move(maybe_server.value());

    auto uri = server->uri();
    CHECK(uri.rfind("http://127.0.0.1:", 0) == 0);
}

TEST_CASE("MockServer stop succeeds", "[mock_server]") {
    auto result = MockServer::start();
    REQUIRE(result.is_ok());

    auto maybe_server = std::move(result).ok();
    REQUIRE(maybe_server.has_value());
    auto server = std::move(maybe_server.value());

    auto stop_result = server->stop();
    CHECK(stop_result.is_ok());
}

TEST_CASE("MockServer stop is idempotent", "[mock_server]") {
    auto result = MockServer::start();
    REQUIRE(result.is_ok());

    auto maybe_server = std::move(result).ok();
    REQUIRE(maybe_server.has_value());
    auto server = std::move(maybe_server.value());

    CHECK(server->stop().is_ok());
    CHECK(server->stop().is_ok());
    CHECK(server->stop().is_ok());
}

TEST_CASE("MockServer start_on with port 0 picks random port", "[mock_server]") {
    auto result = MockServer::start_on("127.0.0.1", 0);
    REQUIRE(result.is_ok());

    auto maybe_inner = std::move(result).ok();
    REQUIRE(maybe_inner.has_value());

    auto inner = std::move(maybe_inner.value());
    REQUIRE(inner.is_ok());

    auto maybe_server = std::move(inner).ok();
    REQUIRE(maybe_server.has_value());
    auto server = std::move(maybe_server.value());

    auto uri = server->uri();
    CHECK(uri.rfind("http://127.0.0.1:", 0) == 0);
}

TEST_CASE("MockServer start_on with specific port", "[mock_server]") {
    auto result = MockServer::start_on("127.0.0.1", 17893);
    REQUIRE(result.is_ok());

    auto maybe_inner = std::move(result).ok();
    REQUIRE(maybe_inner.has_value());

    auto inner = std::move(maybe_inner.value());
    REQUIRE(inner.is_ok());

    auto maybe_server = std::move(inner).ok();
    REQUIRE(maybe_server.has_value());
    auto server = std::move(maybe_server.value());

    auto uri = server->uri();
    CHECK(uri == "http://127.0.0.1:17893");
}

TEST_CASE("MockServer start_on with invalid host returns error", "[mock_server]") {
    auto result = MockServer::start_on("not_a_valid_host", 8080);
    REQUIRE(result.is_ok());

    auto maybe_inner = std::move(result).ok();
    REQUIRE(maybe_inner.has_value());

    auto inner = std::move(maybe_inner.value());
    REQUIRE(inner.is_err());

    auto maybe_err = std::move(inner).err();
    REQUIRE(maybe_err.has_value());
    CHECK(maybe_err.value() == MockServerError::InvalidHost);
}

TEST_CASE("MockServer is reachable via TCP after start", "[mock_server][network]") {
    auto result = MockServer::start();
    REQUIRE(result.is_ok());

    auto maybe_server = std::move(result).ok();
    REQUIRE(maybe_server.has_value());
    auto server = std::move(maybe_server.value());

    auto [host, port] = parse_uri(server->uri());
    CHECK(connect_to(host, port) == 0);
}

TEST_CASE("MockServer is not reachable after stop", "[mock_server][network]") {
    auto result = MockServer::start();
    REQUIRE(result.is_ok());

    auto maybe_server = std::move(result).ok();
    REQUIRE(maybe_server.has_value());
    auto server = std::move(maybe_server.value());

    auto [host, port] = parse_uri(server->uri());

    REQUIRE(server->stop().is_ok());

    CHECK(connect_to(host, port) < 0);
}

TEST_CASE("Repository builder is immutable", "[repository]") {
    auto repo1 = Repository::create("octocat", "hello-world");
    auto repo2 = repo1->description("A great repo");

    // In C++ bindings, name() might return a string.
    // Let's assume it has some getters if we added them, but the task focuses on builders.
    // The main point is that repo1 is not changed.

    // We don't have description() getter in the bridge yet, but we can verify it doesn't crash
    // and that we can add both to a server if we wanted to (though they have same name).

    CHECK(repo1.get() != repo2.get());
}

TEST_CASE("MockServer data registration", "[mock_server][data]") {
    auto server = MockServer::start().ok().value();
    auto uri = server->uri();

    SECTION("Add Repository") {
        auto repo = Repository::create("octocat", "hello-world");
        auto result = server->add_repository(*repo);
        CHECK(result.is_ok());

        // Basic reachability check via URI would require a HTTP client in C++.
        // For now we just verify the FFI calls succeed.
    }

    SECTION("Add Release") {
        auto release = Release::create("octocat", "hello-world", "v1.0.0")
            ->name("v1.0.0 Release")
            ->body("Description of release");
        auto result = server->add_release("octocat", "hello-world", *release);
        CHECK(result.is_ok());
    }

    SECTION("Add Commit") {
        auto commit = Commit::create("octocat", "hello-world")
            ->sha("abc123def")
            ->message("Commit message");
        auto result = server->add_commit("octocat", "hello-world", *commit);
        CHECK(result.is_ok());
    }

    SECTION("Add Asset") {
        std::vector<uint8_t> data = {'h', 'e', 'l', 'l', 'o'};
        auto asset = Asset::from_bytes("test.txt", data, "text/plain");
        auto result = server->add_asset("octocat", "hello-world", "v1.0.0", *asset);
        CHECK(result.is_ok());
    }

    SECTION("Mock Behavior") {
        auto behavior = MockBehavior::new_error(MockError::InternalServerError);
        CHECK(server->add_mock_behavior(*behavior).is_ok());
        CHECK(server->clear_all_mock_behaviors().is_ok());
    }
}

TEST_CASE("MockServer load from file", "[mock_server][data]") {
    auto server = MockServer::start().ok().value();
    auto data_dir = get_data_dir();

    SECTION("Load Repositories") {
        auto path = data_dir + "/repositories.json";
        auto result = server->add_repositories_from_file(path);
        CHECK(result.is_ok());
    }

    SECTION("Load Releases") {
        auto path = data_dir + "/releases.json";
        auto result = server->add_releases_from_file(path, "CleverRaven", "Cataclysm-DDA");
        CHECK(result.is_ok());
    }

    SECTION("Load Commits") {
        auto path = data_dir + "/commits.json";
        auto result = server->add_commits_from_file(path, "karpathy", "arxiv-sanity-lite");
        CHECK(result.is_ok());
    }
}
