require "spec_helper"
require "net/http"
require "json"

RSpec.describe GitHubMockAPI::Release do
  describe ".new" do
    it "creates a new Release" do
      release = described_class.new("octocat", "hello-world", "v1.0.0")
      expect(release).to be_a(described_class)
    end
  end

  describe "builder methods" do
    it "supports method chaining and mutates self" do
      release = described_class.new("octocat", "hello-world", "v1.0.0")
      expect(release.name("v1.0.0")).to eq(release)
      expect(release.body("Release notes")).to eq(release)
      expect(release.target_commitish("main")).to eq(release)
      expect(release.draft(true)).to eq(release)
      expect(release.prerelease(false)).to eq(release)
      expect(release.created_at("2023-01-01T00:00:00Z")).to eq(release)
    end
  end

  describe "integration with MockServer" do
    let(:server) { GitHubMockAPI::MockServer.start }
    after { server.stop }

    it "can be registered with the server and queried" do
      release = described_class.new("test-owner", "test-repo", "v2.0.0")
        .name("Second Release")
        .body("Improved performance")
        .prerelease(true)

      server.add_release(release)

      uri = URI("#{server.uri}/repos/test-owner/test-repo/releases")
      response = Net::HTTP.get_response(uri)

      expect(response.code).to eq("200")
      data = JSON.parse(response.body)
      expect(data).to be_an(Array)
      expect(data.length).to eq(1)

      rel = data[0]
      expect(rel["tag_name"]).to eq("v2.0.0")
      expect(rel["name"]).to eq("Second Release")
      expect(rel["body"]).to eq("Improved performance")
      expect(rel["prerelease"]).to eq(true)
    end
  end
end
