use magnus::{Error, Value};

#[test]
fn ruby_mock_server_smoke_test() -> Result<(), Error> {
    let ruby = unsafe { magnus::embed::init() };
    github_mock_api_ruby::init(&ruby)?;

    let _: Value = ruby.eval(
        r#"
          server = GitHubMockAPI::MockServer.start
          raise "unexpected uri: #{server.uri}" unless server.uri.start_with?("http://127.0.0.1:")
          server.stop
          server.stop

          explicit = GitHubMockAPI::MockServer.start_on("127.0.0.1", 0)
          raise "unexpected explicit uri: #{explicit.uri}" unless explicit.uri.start_with?("http://127.0.0.1:")
          explicit.stop

          begin
            GitHubMockAPI::MockServer.start_on("not-an-ip", 0)
            raise "invalid host did not raise"
          rescue ArgumentError
          end
        "#,
    )?;

    Ok(())
}
