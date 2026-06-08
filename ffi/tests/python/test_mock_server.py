"""Tests for the github_mock_api Python bindings."""

import pytest
import requests
from github_mock_api import MockServer, MockBehavior, MockError


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

        response = requests.get(f"{server.uri()}/repos/owner/repo")
        assert response.status_code == 500
        assert response.json()["message"] == "Internal Server Error"

        server.clear_all_mock_behaviors()
        response = requests.get(f"{server.uri()}/repos/owner/repo")
        # Should be 404 because the repo doesn't exist, but not 500
        assert response.status_code == 404

    def test_mock_behavior_rate_limit_exceeded(self, server: MockServer) -> None:
        behavior = MockBehavior.builder().error(MockError.RATE_LIMIT_EXCEEDED).build()
        server.add_mock_behavior(behavior)

        response = requests.get(f"{server.uri()}/repos/owner/repo")
        assert response.status_code == 403
        assert response.json()["message"] == "API rate limit exceeded"

    def test_mock_behavior_builder_immutability(self) -> None:
        builder1 = MockBehavior.builder()
        builder2 = builder1.error(MockError.INTERNAL_SERVER_ERROR)
        assert builder1 is not builder2

        behavior1 = builder1.build()
        behavior2 = builder2.build()
        assert behavior1 is not behavior2
