require "spec_helper"
require "net/http"
require "json"

RSpec.describe GitHubMockAPI::Repository do
  describe ".new" do
    it "creates a new Repository" do
      repo = described_class.new("octocat", "hello-world")
      expect(repo).to be_a(described_class)
    end
  end

  describe "builder methods" do
    it "supports method chaining and mutates self" do
      repo = described_class.new("octocat", "hello-world")
      expect(repo.description("A test repo")).to eq(repo)
      expect(repo.private(true)).to eq(repo)
      expect(repo.stargazers_count(42)).to eq(repo)
      expect(repo.default_branch("develop")).to eq(repo)
    end
  end

  describe "integration with MockServer" do
    let(:server) { GitHubMockAPI::MockServer.start }
    after { server.stop }

    it "can be registered with the server and queried" do
      repo = described_class.new("test-owner", "test-repo")
        .description("Test description")
        .stargazers_count(100)

      server.add_repository(repo)

      uri = URI("#{server.uri}/repos/test-owner/test-repo")
      response = Net::HTTP.get_response(uri)

      expect(response.code).to eq("200")
      data = JSON.parse(response.body)
      expect(data["name"]).to eq("test-repo")
      expect(data["owner"]["login"]).to eq("test-owner")
      expect(data["description"]).to eq("Test description")
      expect(data["stargazers_count"]).to eq(100)
    end

    it "is case-insensitive for owner and repo lookup" do
      repo = described_class.new("Owner", "Repo")
      server.add_repository(repo)

      uri = URI("#{server.uri}/repos/owner/repo")
      response = Net::HTTP.get_response(uri)
      expect(response.code).to eq("200")
    end
  end
end
