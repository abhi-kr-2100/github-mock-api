"""Tests for the github_mock_api Python bindings."""

import pytest
import requests
import os
from github_mock_api import MockServer, Repository, Release, Commit, Asset, MockBehavior, MockError


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

    @pytest.fixture
    def data_dir(self) -> str:
        # Adjusted path to find testing/data relative to the workspace root
        return os.path.abspath(os.path.join(os.path.dirname(__file__), "../../../testing/data"))

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

    def test_add_repository(self, server: MockServer) -> None:
        repo = Repository("octocat", "hello-world")
        server.add_repository(repo)

        resp = requests.get(f"{server.uri()}/repos/octocat/hello-world")
        assert resp.status_code == 200
        data = resp.json()
        assert data["name"] == "hello-world"
        assert data["owner"]["login"] == "octocat"

    def test_release_builder_is_immutable(self) -> None:
        release1 = Release("octocat", "hello-world", "v1.0.0")
        release2 = release1.name("v1.0.0 Release")
        assert release1 is not release2
        # Verify release1 hasn't changed (though we don't have getters,
        # we can check it doesn't have the name if we had a way,
        # but the identity check is a good start)

    def test_add_release(self, server: MockServer) -> None:
        release = Release("octocat", "hello-world", "v1.0.0")
        release = release.name("v1.0.0 Release")
        server.add_release("octocat", "hello-world", release)

        resp = requests.get(f"{server.uri()}/repos/octocat/hello-world/releases/tags/v1.0.0")
        assert resp.status_code == 200
        data = resp.json()
        assert data["tag_name"] == "v1.0.0"
        assert data["name"] == "v1.0.0 Release"

    def test_commit_builder_is_immutable(self) -> None:
        commit1 = Commit("octocat", "hello-world")
        commit2 = commit1.sha("abc123def456")
        assert commit1 is not commit2

    def test_add_commit(self, server: MockServer) -> None:
        commit = Commit("octocat", "hello-world")
        commit = commit.sha("abc123def456")
        commit = commit.message("Initial commit")
        server.add_commit("octocat", "hello-world", commit)

        resp = requests.get(f"{server.uri()}/repos/octocat/hello-world/commits/abc123def456")
        assert resp.status_code == 200
        data = resp.json()
        assert data["sha"] == "abc123def456"
        assert data["commit"]["message"] == "Initial commit"

    def test_add_asset(self, server: MockServer) -> None:
        asset = Asset.from_bytes("test.txt", b"hello world", "text/plain")
        server.add_asset("octocat", "hello-world", "v1.0.0", asset)

        resp = requests.get(f"{server.uri()}/octocat/hello-world/releases/download/v1.0.0/test.txt")
        assert resp.status_code == 200
        assert resp.content == b"hello world"
        assert resp.headers["Content-Type"] == "text/plain"

    def test_mock_behavior_error(self, server: MockServer) -> None:
        behavior = MockBehavior.error(MockError.InternalServerError)
        server.add_mock_behavior(behavior)

        resp = requests.get(f"{server.uri()}/repos/any/repo")
        assert resp.status_code == 500

        server.clear_all_mock_behaviors()
        resp = requests.get(f"{server.uri()}/repos/any/repo")
        assert resp.status_code == 404

    def test_load_from_file(self, server: MockServer, data_dir: str) -> None:
        # Repositories
        repos_file = os.path.join(data_dir, "repositories.json")
        repos = Repository.load_from_file(repos_file)
        for r in repos:
            server.add_repository(r)

        resp = requests.get(f"{server.uri()}/repos/karpathy/arxiv-sanity-lite")
        assert resp.status_code == 200
        assert resp.json()["name"] == "arxiv-sanity-lite"

        # Releases
        releases_file = os.path.join(data_dir, "releases.json")
        releases = Release.load_from_file(releases_file, "CleverRaven", "Cataclysm-DDA")
        for r in releases:
            server.add_release("CleverRaven", "Cataclysm-DDA", r)

        resp = requests.get(f"{server.uri()}/repos/CleverRaven/Cataclysm-DDA/releases")
        assert resp.status_code == 200
        assert len(resp.json()) >= 1
        assert resp.json()[0]["tag_name"] == "cdda-experimental-2026-06-04-1344"
