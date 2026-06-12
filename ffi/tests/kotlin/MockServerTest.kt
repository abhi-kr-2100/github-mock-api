fun main() {
    try {
        val server = io.github.abhi_kr_2100.github_mock_api_ffi.MockServer.start().getOrThrow()

        val uri = server.uri()
        if (!uri.startsWith("http://127.0.0.1:")) {
            System.err.println("unexpected uri: $uri")
            try { server.stop() } catch (_: Exception) {}
            System.exit(1)
        }

        val repo = io.github.abhi_kr_2100.github_mock_api_ffi.Repository.new_("octocat", "hello-world")
            .withDescription("A test repository")
            .withPrivate(true)
            .withStargazersCount(42uL)
            .withDefaultBranch("develop")

        server.addRepository(repo).getOrThrow()

        // Verify Repository with HTTP
        val repoUrl = java.net.URL("$uri/repos/octocat/hello-world")
        val repoConnection = repoUrl.openConnection() as java.net.HttpURLConnection
        repoConnection.requestMethod = "GET"
        if (repoConnection.responseCode != 200) {
            System.err.println("expected 200 OK for repo, got ${repoConnection.responseCode}")
            System.exit(1)
        }

        val commit = io.github.abhi_kr_2100.github_mock_api_ffi.Commit.new_("octocat", "hello-world")
            .withSha("abc123def456")
            .withMessage("A test commit")
            .withAuthorName("Test User")
            .withAuthorEmail("test@example.com")

        server.addCommit(commit).getOrThrow()

        // Verify Commit with HTTP
        val commitUrl = java.net.URL("$uri/repos/octocat/hello-world/commits/abc123def456")
        val commitConnection = commitUrl.openConnection() as java.net.HttpURLConnection
        commitConnection.requestMethod = "GET"
        if (commitConnection.responseCode != 200) {
            System.err.println("expected 200 OK for commit, got ${commitConnection.responseCode}")
            System.exit(1)
        }

        server.stop()
    } catch (e: Exception) {
        e.printStackTrace()
        System.err.println("MockServer test failed: $e")
        System.exit(1)
    }
}
