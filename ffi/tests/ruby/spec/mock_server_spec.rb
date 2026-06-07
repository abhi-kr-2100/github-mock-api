require "spec_helper"
require "net/http"
require "json"

RSpec.describe GitHubMockAPI::MockServer do

  after { @server&.stop }

  let(:data_dir) { File.expand_path("../../../../../testing/data", __dir__) }

  describe ".start" do
    it "starts a server and returns a MockServer instance" do
      @server = described_class.start
      expect(@server).to be_a(described_class)
    end

    it "returns a valid URI" do
      @server = described_class.start
      expect(@server.uri).to match(/\Ahttp:\/\/127\.0\.0\.1:\d+\z/)
    end
  end

  describe ".start_on" do
    it "starts a server on the given host and port" do
      @server = described_class.start_on("127.0.0.1", 0)
      expect(@server).to be_a(described_class)
      expect(@server.uri).to match(/\Ahttp:\/\/127\.0\.0\.1:\d+\z/)
    end

    it "raises ArgumentError for an invalid host" do
      expect { described_class.start_on("not-an-ip", 0) }.to raise_error(ArgumentError)
    end
  end

  describe "#stop" do
    it "stops the server without error" do
      @server = described_class.start
      expect { @server.stop }.not_to raise_error
    end

    it "can be called multiple times without error" do
      @server = described_class.start
      @server.stop
      expect { @server.stop }.not_to raise_error
    end
  end

  describe "#uri" do
    it "returns a String" do
      @server = described_class.start
      expect(@server.uri).to be_a(String)
    end

    it "returns an HTTP URL" do
      @server = described_class.start
      expect(@server.uri).to start_with("http://")
    end
  end

  context "with multiple servers" do
    it "starts multiple servers on different ports" do
      s1 = described_class.start
      @server = s1
      s2 = described_class.start
      expect(s1.uri).not_to eq(s2.uri)
    ensure
      s1&.stop
      s2&.stop
    end
  end

  describe "data registration" do
    before { @server = described_class.start }

    it "registers a repository" do
      repo = GitHubMockAPI::Repository.new("octocat", "hello-world")
      @server.add_repository(repo)

      uri = URI("#{@server.uri}/repos/octocat/hello-world")
      resp = Net::HTTP.get_response(uri)
      expect(resp.code).to eq("200")
      data = JSON.parse(resp.body)
      expect(data["name"]).to eq("hello-world")
    end

    it "registers a release" do
      release = GitHubMockAPI::Release.new("octocat", "hello-world", "v1.0.0")
      release = release.name("v1.0.0 Release")
      @server.add_release("octocat", "hello-world", release)

      uri = URI("#{@server.uri}/repos/octocat/hello-world/releases/tags/v1.0.0")
      resp = Net::HTTP.get_response(uri)
      expect(resp.code).to eq("200")
      data = JSON.parse(resp.body)
      expect(data["tag_name"]).to eq("v1.0.0")
      expect(data["name"]).to eq("v1.0.0 Release")
    end

    it "registers a commit" do
      commit = GitHubMockAPI::Commit.new("octocat", "hello-world")
      commit = commit.sha("abc123def456")
      commit = commit.message("Initial commit")
      @server.add_commit("octocat", "hello-world", commit)

      uri = URI("#{@server.uri}/repos/octocat/hello-world/commits/abc123def456")
      resp = Net::HTTP.get_response(uri)
      expect(resp.code).to eq("200")
      data = JSON.parse(resp.body)
      expect(data["sha"]).to eq("abc123def456")
      expect(data["commit"]["message"]).to eq("Initial commit")
    end

    it "registers an asset" do
      asset = GitHubMockAPI::Asset.from_bytes("test.txt", "hello world".bytes, "text/plain")
      @server.add_asset("octocat", "hello-world", "v1.0.0", asset)

      uri = URI("#{@server.uri}/octocat/hello-world/releases/download/v1.0.0/test.txt")
      resp = Net::HTTP.get_response(uri)
      expect(resp.code).to eq("200")
      expect(resp.body).to eq("hello world")
      expect(resp["Content-Type"]).to eq("text/plain")
    end

    it "applies mock behaviors" do
      behavior = GitHubMockAPI::MockBehavior.error(:internal_server_error)
      @server.add_mock_behavior(behavior)

      uri = URI("#{@server.uri}/repos/any/repo")
      resp = Net::HTTP.get_response(uri)
      expect(resp.code).to eq("500")

      @server.clear_all_mock_behaviors
      resp = Net::HTTP.get_response(uri)
      expect(resp.code).to eq("404")
    end

    it "loads data from files" do
      repos_file = File.expand_path("../../../../../testing/data/repositories.json", __FILE__)
      repos = GitHubMockAPI::Repository.load_from_file(repos_file)
      repos.each { |r| @server.add_repository(r) }

      uri = URI("#{@server.uri}/repos/karpathy/arxiv-sanity-lite")
      resp = Net::HTTP.get_response(uri)
      expect(resp.code).to eq("200")
      expect(JSON.parse(resp.body)["name"]).to eq("arxiv-sanity-lite")
    end
  end
end
