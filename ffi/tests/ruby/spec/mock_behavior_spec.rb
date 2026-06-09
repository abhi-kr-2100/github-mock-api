require "spec_helper"
require "net/http"
require "json"

RSpec.describe "Mock Behavior" do
  before(:all) do
    @server = GitHubMockAPI::MockServer.start
  end

  after(:all) do
    @server.stop
  end

  before(:each) do
    @server.clear_all_mock_behaviors
  end

  it "returns internal server error when configured" do
    behavior = GitHubMockAPI::MockBehavior.new.error(:internal_server_error)
    @server.add_mock_behavior(behavior)

    uri = URI("#{@server.uri}/repos/owner/repo")
    response = Net::HTTP.get_response(uri)

    expect(response.code).to eq("500")
    body = JSON.parse(response.body)
    expect(body["message"]).to eq("Internal Server Error")
  end

  it "returns rate limit exceeded when configured" do
    behavior = GitHubMockAPI::MockBehavior.new.error(:rate_limit_exceeded)
    @server.add_mock_behavior(behavior)

    uri = URI("#{@server.uri}/repos/owner/repo")
    response = Net::HTTP.get_response(uri)

    expect(response.code).to eq("403")
    body = JSON.parse(response.body)
    expect(body["message"]).to eq("API rate limit exceeded")
  end

  it "can clear behaviors" do
    behavior = GitHubMockAPI::MockBehavior.new.error(:internal_server_error)
    @server.add_mock_behavior(behavior)

    @server.clear_all_mock_behaviors

    uri = URI("#{@server.uri}/repos/owner/repo")
    response = Net::HTTP.get_response(uri)

    # Should be 404 since repo not added, but not 500
    expect(response.code).to eq("404")
  end

  it "raises ServerStoppedError when server is stopped" do
    temp_server = GitHubMockAPI::MockServer.start
    temp_server.stop

    behavior = GitHubMockAPI::MockBehavior.new.error(:internal_server_error)
    expect { temp_server.add_mock_behavior(behavior) }.to raise_error(GitHubMockAPI::ServerStoppedError)
    expect { temp_server.clear_all_mock_behaviors }.to raise_error(GitHubMockAPI::ServerStoppedError)
  end

  it "raises ArgumentError for invalid error symbols" do
    expect { GitHubMockAPI::MockBehavior.new.error(:invalid_error) }.to raise_error(ArgumentError)
  end

  it "raises error on conflicting global behaviors" do
    behavior1 = GitHubMockAPI::MockBehavior.new.error(:internal_server_error)
    behavior2 = GitHubMockAPI::MockBehavior.new.error(:rate_limit_exceeded)

    @server.add_mock_behavior(behavior1)
    expect { @server.add_mock_behavior(behavior2) }.to raise_error(RuntimeError, /A global error behavior is already set/)
  end
end
