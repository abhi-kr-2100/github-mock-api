require "spec_helper"
require "net/http"
require "json"

RSpec.describe GitHubMockAPI::Commit do
  let(:owner) { "octocat" }
  let(:repo) { "hello-world" }

  describe ".new" do
    it "creates a new Commit instance" do
      commit = described_class.new(owner, repo)
      expect(commit).to be_a(described_class)
    end
  end

  describe "method chaining" do
    it "supports chaining for all builder methods and returns self" do
      commit = described_class.new(owner, repo)

      result = commit
        .sha("abc123def")
        .message("feat: add new feature")
        .author_name("Mona")
        .author_email("mona@github.com")

      expect(result).to eq(commit)
    end
  end

  describe "integration with MockServer" do
    let(:server) { GitHubMockAPI::MockServer.start }
    after { server.stop }

    it "can be added to the mock server" do
      commit = described_class.new(owner, repo)
        .sha("abc123def")
        .message("Initial commit")

      expect { server.add_commit(commit) }.not_to raise_error

      uri = URI("#{server.uri}/repos/#{owner}/#{repo}/commits/abc123def")
      response = Net::HTTP.get_response(uri)

      expect(response.code).to eq("200")
      data = JSON.parse(response.body)
      expect(data["sha"]).to eq("abc123def")
      expect(data["commit"]["message"]).to eq("Initial commit")
    end
  end
end
