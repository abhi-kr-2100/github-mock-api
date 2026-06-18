fun main() {
    try {
        val server = io.github.abhi_kr_2100.github_mock_api_ffi.MockServer.start().getOrThrow()

        val uri = server.uri()
        if (!uri.startsWith("http://127.0.0.1:")) {
            System.err.println("unexpected uri: $uri")
            try { server.stop() } catch (_: Exception) {}
            System.exit(1)
        }

        val repo = io.github.abhi_kr_2100.github_mock_api_ffi.Repository.new_("octocat", "hello-world").getOrThrow()
            .withDescription("A test repository").getOrThrow()
            .withPrivate(true)
            .withStargazersCount(42uL)
            .withDefaultBranch("develop").getOrThrow()

        server.addRepository(repo).getOrThrow()

        // Verify Repository with HTTP
        val repoUrl = java.net.URL("$uri/repos/octocat/hello-world")
        val repoConnection = repoUrl.openConnection() as java.net.HttpURLConnection
        repoConnection.requestMethod = "GET"
        if (repoConnection.responseCode != 200) {
            System.err.println("expected 200 OK for repo, got ${repoConnection.responseCode}")
            System.exit(1)
        }

        val repoClear = io.github.abhi_kr_2100.github_mock_api_ffi.Repository.new_("octocat", "hello-world").getOrThrow()
            .withDescription("To be cleared").getOrThrow()
            .withClearDescription()
        server.addRepository(repoClear).getOrThrow()

        val repoClearUrl = java.net.URL("$uri/repos/octocat/hello-world")
        val repoClearConnection = repoClearUrl.openConnection() as java.net.HttpURLConnection
        repoClearConnection.requestMethod = "GET"
        val repoClearBody = repoClearConnection.inputStream.bufferedReader().readText()
        if (repoClearBody.contains("\"description\":\"")) {
            System.err.println("expected null description in $repoClearBody")
            System.exit(1)
        }

        val commit = io.github.abhi_kr_2100.github_mock_api_ffi.Commit.new_("octocat", "hello-world").getOrThrow()
            .withSha("abc123def456").getOrThrow()
            .withMessage("A test commit").getOrThrow()
            .withAuthorName("Test User").getOrThrow()
            .withAuthorEmail("test@example.com").getOrThrow()

        server.addCommit(commit).getOrThrow()

        // Verify Commit with HTTP
        val commitUrl = java.net.URL("$uri/repos/octocat/hello-world/commits/abc123def456")
        val commitConnection = commitUrl.openConnection() as java.net.HttpURLConnection
        commitConnection.requestMethod = "GET"
        if (commitConnection.responseCode != 200) {
            System.err.println("expected 200 OK for commit, got ${commitConnection.responseCode}")
            System.exit(1)
        }

        val asset = io.github.abhi_kr_2100.github_mock_api_ffi.Asset.fromBytes("test.txt", "hello world".toByteArray().toUByteArray(), "text/plain").getOrThrow()
        server.addAsset("octocat", "hello-world", "v1.0.0", asset).getOrThrow()

        // Verify Asset with HTTP
        val assetUrl = java.net.URL("$uri/octocat/hello-world/releases/download/v1.0.0/test.txt")
        val assetConnection = assetUrl.openConnection() as java.net.HttpURLConnection
        assetConnection.requestMethod = "GET"
        if (assetConnection.responseCode != 200) {
            System.err.println("expected 200 OK for asset, got ${assetConnection.responseCode}")
            System.exit(1)
        }
        val assetBody = assetConnection.inputStream.bufferedReader().readText()
        if (assetBody != "hello world") {
            System.err.println("expected 'hello world' for asset body, got '$assetBody'")
            System.exit(1)
        }

        server.stop()
    } catch (e: Exception) {
        e.printStackTrace()
        System.err.println("MockServer test failed: $e")
        System.exit(1)
    }
}
