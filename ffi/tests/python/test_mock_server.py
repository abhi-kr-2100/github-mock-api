"""Tests for the github_mock_api Python bindings."""

import json
import urllib.error
import urllib.request

import pytest
from github_mock_api import MockServer, MockBehavior, MockError, Repository


class TestMockServer:
    @pytest.fixture
    def server(self) -> MockServer:
        _server = MockServer.start()
        yield _server
        _server.stop()

    @pytest.fixture
    def servers(self) -> list[MockServer]:
        _servers: list[MockServer] = []
        yield _servers
        for s in _servers:
            s.stop()

    def test_start(self, server: MockServer) -> None:
        uri = server.uri()
        assert uri.startswith("http://127.0.0.1:")

    def test_start_on(self, servers: list[MockServer]) -> None:
        server = MockServer.start_on("127.0.0.1", 0)
        servers.append(server)
        uri = server.uri()
        assert uri.startswith("http://127.0.0.1:")
        server.stop()

    def test_stop_is_idempotent(self, server: MockServer) -> None:
        server.stop()
        server.stop()

    def test_uri_accessible_after_stop(self, server: MockServer) -> None:
        uri = server.uri()
        server.stop()
        assert server.uri() == uri

    def test_start_on_invalid_host_raises_value_error(self) -> None:
        with pytest.raises(ValueError):
            MockServer.start_on("not-an-ip", 0)

    def test_multiple_servers(self, servers: list[MockServer]) -> None:
        server1 = MockServer.start()
        server2 = MockServer.start()
        servers.append(server1)
        servers.append(server2)
        assert server1.uri() != server2.uri()
        server1.stop()
        server2.stop()

    def test_start_on_exact_port(self, servers: list[MockServer]) -> None:
        server = MockServer.start_on("127.0.0.1", 12346)
        servers.append(server)
        assert server.uri() == "http://127.0.0.1:12346"
        server.stop()

    def test_mock_behavior_internal_server_error(self, server: MockServer) -> None:
        behavior = MockBehavior.builder().error(MockError.INTERNAL_SERVER_ERROR).build()
        server.add_mock_behavior(behavior)

        url = f"{server.uri()}/repos/owner/repo"
        with pytest.raises(urllib.error.HTTPError) as excinfo:
            urllib.request.urlopen(url)
        assert excinfo.value.code == 500
        body = json.loads(excinfo.value.read().decode())
        assert body["message"] == "Internal Server Error"

        server.clear_all_mock_behaviors()
        with pytest.raises(urllib.error.HTTPError) as excinfo:
            urllib.request.urlopen(url)
        # Should be 404 because the repo doesn't exist, but not 500
        assert excinfo.value.code == 404

    def test_mock_behavior_rate_limit_exceeded(self, server: MockServer) -> None:
        behavior = MockBehavior.builder().error(MockError.RATE_LIMIT_EXCEEDED).build()
        server.add_mock_behavior(behavior)

        url = f"{server.uri()}/repos/owner/repo"
        with pytest.raises(urllib.error.HTTPError) as excinfo:
            urllib.request.urlopen(url)
        assert excinfo.value.code == 403
        body = json.loads(excinfo.value.read().decode())
        assert body["message"] == "API rate limit exceeded"

    def test_mock_behavior_builder_immutability(self) -> None:
        builder1 = MockBehavior.builder()
        builder2 = builder1.error(MockError.INTERNAL_SERVER_ERROR)
        assert builder1 is not builder2

        behavior1 = builder1.build()
        behavior2 = builder2.build()
        assert behavior1 is not behavior2

    def test_add_repository(self, server: MockServer) -> None:
        repo = (
            Repository.builder("octocat", "hello-world")
            .description("A test repository")
            .stargazers_count(42)
            .build()
        )
        server.add_repository(repo)

        url = f"{server.uri()}/repos/octocat/hello-world"
        with urllib.request.urlopen(url) as response:
            assert response.status == 200
            body = json.loads(response.read().decode())
            assert body["name"] == "hello-world"
            assert body["owner"]["login"] == "octocat"
            assert body["description"] == "A test repository"
            assert body["stargazers_count"] == 42
