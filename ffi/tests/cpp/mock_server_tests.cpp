#include <memory>
#include <optional>
#include <string>
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

using namespace github_mock_api_ffi;

namespace {

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

TEST_CASE("MockServer can add a repository", "[mock_server]") {
    auto server = MockServer::start().ok().value();

    auto repo = Repository::new_("octocat", "hello-world").ok().value();
    auto repo2 = repo->with_description("A test repository").ok().value();
    auto repo3 = repo2->with_private(true);
    auto repo4 = repo3->with_stargazers_count(42);
    auto repo5 = repo4->with_default_branch("develop").ok().value();

    auto add_result = server->add_repository(*repo5);
    CHECK(add_result.is_ok());
}

TEST_CASE("MockServer serves added repository", "[mock_server][network]") {
    auto server = MockServer::start().ok().value();

    auto repo = Repository::new_("octocat", "hello-world").ok().value();

    auto add_result = server->add_repository(*repo);
    REQUIRE(add_result.is_ok());

    auto uri = server->uri() + "/repos/octocat/hello-world";
    std::string cmd = "curl -s -f " + uri + " > /dev/null";

    CHECK(system(cmd.c_str()) == 0);
}

TEST_CASE("MockServer can add a commit", "[mock_server]") {
    auto server = MockServer::start().ok().value();

    auto commit = Commit::new_("octocat", "hello-world").ok().value();
    auto commit2 = commit->with_message("A test commit").ok().value();
    auto commit3 = commit2->with_sha("abc123def456").ok().value();
    auto commit4 = commit3->with_author_name("Test User").ok().value();
    auto commit5 = commit4->with_author_email("test@example.com").ok().value();

    auto add_result = server->add_commit(*commit5);
    CHECK(add_result.is_ok());
}

TEST_CASE("MockServer serves added commit", "[mock_server][network]") {
    auto server = MockServer::start().ok().value();

    auto commit = Commit::new_("octocat", "hello-world").ok().value();
    auto commit2 = commit->with_sha("abc123def456").ok().value();

    auto add_result = server->add_commit(*commit2);
    REQUIRE(add_result.is_ok());

    auto uri = server->uri() + "/repos/octocat/hello-world/commits/abc123def456";
    std::string cmd = "curl -s -f " + uri + " > /dev/null";

    CHECK(system(cmd.c_str()) == 0);
}

TEST_CASE("MockServer can add a release", "[mock_server]") {
    auto server = MockServer::start().ok().value();

    auto release = Release::new_("octocat", "hello-world", "v1.0.0").ok().value();
    auto release2 = release->with_name("Version 1.0.0").ok().value();
    auto release3 = release2->with_body("Initial release").ok().value();
    auto release4 = release3->with_target_commitish("main").ok().value();
    auto release5 = release4->with_draft(false);
    auto release6 = release5->with_prerelease(false);

    auto add_result = server->add_release(*release6);
    CHECK(add_result.is_ok());
}

TEST_CASE("MockServer serves added release", "[mock_server][network]") {
    auto server = MockServer::start().ok().value();

    auto release = Release::new_("octocat", "hello-world", "v1.0.0").ok().value();

    auto add_result = server->add_release(*release);
    REQUIRE(add_result.is_ok());

    auto uri = server->uri() + "/repos/octocat/hello-world/releases/tags/v1.0.0";
    std::string cmd = "curl -s -f " + uri + " > /dev/null";

    CHECK(system(cmd.c_str()) == 0);
}
