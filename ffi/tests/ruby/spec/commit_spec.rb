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
        .author_date("2023-01-01T00:00:00Z")
        .committer_name("Hubot")
        .committer_email("hubot@github.com")
        .committer_date("2023-01-02T00:00:00Z")

      expect(result).to eq(commit)
    end
  end

  describe ".load_from_file" do
    it "loads commits from a JSON file" do
      path = File.expand_path("../../../../../testing/data/commits.json", __FILE__)
      commits = described_class.load_from_file(path, owner, repo)

      expect(commits).to be_an(Array)
      expect(commits.length).to eq(30)
      expect(commits.first).to be_a(described_class)
    end

    it "raises an error for non-existent files" do
      expect {
        described_class.load_from_file("non_existent.json", owner, repo)
      }.to raise_error(IOError)
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

    it "can configure committer and dates" do
      commit = described_class.new(owner, repo)
        .sha("abc123def")
        .author_name("Mona")
        .author_date("2023-01-01T00:00:00Z")
        .committer_name("Hubot")
        .committer_email("hubot@github.com")
        .committer_date("2023-01-02T00:00:00Z")

      server.add_commit(commit)

      uri = URI("#{server.uri}/repos/#{owner}/#{repo}/commits/abc123def")
      response = Net::HTTP.get_response(uri)
      data = JSON.parse(response.body)

      expect(data["commit"]["author"]["name"]).to eq("Mona")
      expect(data["commit"]["author"]["date"]).to eq("2023-01-01T00:00:00Z")
      expect(data["commit"]["committer"]["name"]).to eq("Hubot")
      expect(data["commit"]["committer"]["email"]).to eq("hubot@github.com")
      expect(data["commit"]["committer"]["date"]).to eq("2023-01-02T00:00:00Z")
      expect(data["committer"]["login"]).to eq(owner) # Top level committer login is set from owner in Mock implementation
    end

    it "can register loaded commits with the server" do
      path = File.expand_path("../../../../../testing/data/commits.json", __FILE__)
      commits = described_class.load_from_file(path, "karpathy", "arxiv-sanity-lite")
      commits.each { |c| server.add_commit(c) }

      uri = URI("#{server.uri}/repos/karpathy/arxiv-sanity-lite/commits")
      response = Net::HTTP.get_response(uri)
      expect(response.code).to eq("200")
      data = JSON.parse(response.body)
      expect(data).to be_an(Array)
      expect(data.length).to eq(30)
      expect(data.first["sha"]).to eq("9291e608e354242c8ff12d47896799d456719922")
    end
  end
end
