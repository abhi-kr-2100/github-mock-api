"""Tests for the github_mock_api Python bindings."""

import json
import os
import urllib.error
import urllib.request

import pytest
from github_mock_api import MockServer, MockBehavior, MockError, Repository, Release, Commit, Asset


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

    def test_add_repository_with_counts(self, server: MockServer) -> None:
        repo = (
            Repository.builder("octocat", "hello-world")
            .subscribers_count(10)
            .network_count(5)
            .build()
        )
        server.add_repository(repo)

        url = f"{server.uri()}/repos/octocat/hello-world"
        with urllib.request.urlopen(url) as response:
            assert response.status == 200
            body = json.loads(response.read().decode())
            assert body["subscribers_count"] == 10
            assert body["network_count"] == 5

    def test_add_release(self, server: MockServer) -> None:
        release = (
            Release.builder("octocat", "hello-world", "v1.0.0")
            .name("First Release")
            .body("Description of the release")
            .build()
        )
        server.add_release("octocat", "hello-world", release)

        url = f"{server.uri()}/repos/octocat/hello-world/releases"
        with urllib.request.urlopen(url) as response:
            assert response.status == 200
            body = json.loads(response.read().decode())
            assert len(body) == 1
            assert body[0]["tag_name"] == "v1.0.0"
            assert body[0]["name"] == "First Release"
            assert body[0]["body"] == "Description of the release"

    def test_add_commit(self, server: MockServer) -> None:
        commit = (
            Commit.builder("octocat", "hello-world")
            .sha("1234567890abcdef1234567890abcdef12345678")
            .message("Initial commit")
            .author_name("Mona Octocat")
            .author_email("mona@github.com")
            .build()
        )
        server.add_commit("octocat", "hello-world", commit)

        url = f"{server.uri()}/repos/octocat/hello-world/commits"
        with urllib.request.urlopen(url) as response:
            assert response.status == 200
            body = json.loads(response.read().decode())
            assert len(body) == 1
            assert body[0]["sha"] == "1234567890abcdef1234567890abcdef12345678"
            assert body[0]["commit"]["message"] == "Initial commit"
            assert body[0]["commit"]["author"]["name"] == "Mona Octocat"
            assert body[0]["commit"]["author"]["email"] == "mona@github.com"

    def test_add_commit_with_committer_and_dates(self, server: MockServer) -> None:
        commit = (
            Commit.builder("octocat", "hello-world")
            .sha("1234567890abcdef1234567890abcdef12345678")
            .message("Initial commit")
            .author_name("Mona Octocat")
            .author_email("mona@github.com")
            .author_date("2023-01-01T00:00:00Z")
            .committer_name("Committer Mona")
            .committer_email("committer@github.com")
            .committer_date("2023-01-02T00:00:00Z")
            .build()
        )
        server.add_commit("octocat", "hello-world", commit)

        url = f"{server.uri()}/repos/octocat/hello-world/commits"
        with urllib.request.urlopen(url) as response:
            assert response.status == 200
            body = json.loads(response.read().decode())
            assert len(body) == 1
            assert body[0]["sha"] == "1234567890abcdef1234567890abcdef12345678"
            assert body[0]["commit"]["message"] == "Initial commit"
            assert body[0]["commit"]["author"]["name"] == "Mona Octocat"
            assert body[0]["commit"]["author"]["email"] == "mona@github.com"
            assert body[0]["commit"]["author"]["date"] == "2023-01-01T00:00:00Z"
            assert body[0]["commit"]["committer"]["name"] == "Committer Mona"
            assert body[0]["commit"]["committer"]["email"] == "committer@github.com"
            assert body[0]["commit"]["committer"]["date"] == "2023-01-02T00:00:00Z"

    def test_add_asset(self, server: MockServer) -> None:
        content = b"hello world"
        asset = Asset.from_bytes("test.txt", content, "text/plain")
        server.add_asset("octocat", "hello-world", "v1.0.0", asset)

        url = f"{server.uri()}/octocat/hello-world/releases/download/v1.0.0/test.txt"
        with urllib.request.urlopen(url) as response:
            assert response.status == 200
            assert response.read() == content
            assert response.headers["Content-Type"] == "text/plain"

    def test_load_repositories_from_file(self, server: MockServer) -> None:
        path = os.path.join(os.path.dirname(__file__), "../../..", "testing/data/repositories.json")
        repos = Repository.load_from_file(path)
        assert len(repos) == 30
        server.add_repository(repos[0])

        # From repositories.json, the first repo is karpathy/arxiv-sanity-lite
        url = f"{server.uri()}/repos/karpathy/arxiv-sanity-lite"
        with urllib.request.urlopen(url) as response:
            assert response.status == 200
            body = json.loads(response.read().decode())
            assert body["name"] == "arxiv-sanity-lite"

    def test_load_releases_from_file(self, server: MockServer) -> None:
        path = os.path.join(os.path.dirname(__file__), "../../..", "testing/data/releases.json")
        releases = Release.load_from_file(path, "owner", "repo")
        assert len(releases) == 30
        for r in releases[:5]:
            server.add_release("owner", "repo", r)

        url = f"{server.uri()}/repos/owner/repo/releases"
        with urllib.request.urlopen(url) as response:
            assert response.status == 200
            body = json.loads(response.read().decode())
            assert len(body) == 5
            # From releases.json, the first tag is cdda-experimental-2026-06-04-1344
            assert body[0]["tag_name"] == "cdda-experimental-2026-06-04-1344"

    def test_load_commits_from_file(self, server: MockServer) -> None:
        path = os.path.join(os.path.dirname(__file__), "../../..", "testing/data/commits.json")
        commits = Commit.load_from_file(path, "owner", "repo")
        assert len(commits) == 30
        for c in commits[:5]:
            server.add_commit("owner", "repo", c)

        url = f"{server.uri()}/repos/owner/repo/commits"
        with urllib.request.urlopen(url) as response:
            assert response.status == 200
            body = json.loads(response.read().decode())
            assert len(body) == 5
            # From commits.json, the first sha is 9291e608e354242c8ff12d47896799d456719922
            assert body[0]["sha"] == "9291e608e354242c8ff12d47896799d456719922"
