$LOAD_PATH.unshift(__dir__)
require "github_mock_api_ruby"

begin
  server = GitHubMockAPI::MockServer.start
  unless server.uri.start_with?("http://127.0.0.1:")
    $stderr.puts "unexpected uri: #{server.uri}"
    server.stop rescue nil
    exit 1
  end
  server.stop

  explicit = GitHubMockAPI::MockServer.start_on("127.0.0.1", 0)
  unless explicit.uri.start_with?("http://127.0.0.1:")
    $stderr.puts "unexpected explicit uri: #{explicit.uri}"
    explicit.stop rescue nil
    exit 1
  end
  explicit.stop

  begin
    GitHubMockAPI::MockServer.start_on("not-an-ip", 0)
    $stderr.puts "invalid host did not raise"
    exit 1
  rescue ArgumentError
  end
rescue => e
  $stderr.puts "Ruby API smoke test failed: #{e}"
  exit 1
end
