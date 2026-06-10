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

        // Verify with HTTP
        val url = java.net.URL("$uri/repos/octocat/hello-world")
        val connection = url.openConnection() as java.net.HttpURLConnection
        connection.requestMethod = "GET"
        if (connection.responseCode != 200) {
            System.err.println("expected 200 OK, got ${connection.responseCode}")
            System.exit(1)
        }

        server.stop()
    } catch (e: Exception) {
        e.printStackTrace()
        System.err.println("MockServer test failed: $e")
        System.exit(1)
    }
}
