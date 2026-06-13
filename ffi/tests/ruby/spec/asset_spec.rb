require "spec_helper"
require "net/http"
require "json"

RSpec.describe GitHubMockAPI::Asset do
  describe ".from_bytes" do
    it "creates an asset from bytes" do
      asset = described_class.from_bytes("test.txt", "hello world".bytes, "text/plain")
      expect(asset).to be_a(described_class)
    end
  end

  describe ".from_file" do
    it "creates an asset from a file" do
      # Use a file that is guaranteed to exist in the repo
      asset = described_class.from_file("repo.json", "testing/data/repositories.json", "application/json")
      expect(asset).to be_a(described_class)
    end
  end

  describe "integration with MockServer" do
    let(:server) { GitHubMockAPI::MockServer.start }
    after { server.stop }

    it "can be registered and downloaded" do
      content = "binary data"
      asset = described_class.from_bytes("data.bin", content.bytes, "application/octet-stream")

      server.add_asset("owner", "repo", "v1.0.0", asset)

      uri = URI("#{server.uri}/owner/repo/releases/download/v1.0.0/data.bin")
      response = Net::HTTP.get_response(uri)

      expect(response.code).to eq("200")
      expect(response.body).to eq(content)
      expect(response["Content-Type"]).to eq("application/octet-stream")
      expect(response["Content-Disposition"]).to include('filename="data.bin"')
    end

    it "is case-insensitive for owner and repo during download" do
      asset = described_class.from_bytes("test.txt", "content".bytes, "text/plain")
      server.add_asset("Owner", "Repo", "v1", asset)

      uri = URI("#{server.uri}/owner/repo/releases/download/v1/test.txt")
      response = Net::HTTP.get_response(uri)
      expect(response.code).to eq("200")
    end

    it "is case-sensitive for tag and filename during download" do
      asset = described_class.from_bytes("test.txt", "content".bytes, "text/plain")
      server.add_asset("owner", "repo", "v1", asset)

      # Wrong tag case
      uri_wrong_tag = URI("#{server.uri}/owner/repo/releases/download/V1/test.txt")
      expect(Net::HTTP.get_response(uri_wrong_tag).code).to eq("404")

      # Wrong filename case
      uri_wrong_file = URI("#{server.uri}/owner/repo/releases/download/v1/Test.txt")
      expect(Net::HTTP.get_response(uri_wrong_file).code).to eq("404")
    end
  end
end
