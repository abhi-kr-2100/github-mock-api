import 'dart:convert';
import 'dart:io';

import 'package:test/test.dart';

import '../../../bindings/dart/lib.g.dart';

void main() {
  group('MockServer', () {
    test('starts on default host and returns valid URI', () {
      final server = MockServer.start();
      addTearDown(server.stop);
      final uri = server.uri();
      expect(uri, startsWith('http://127.0.0.1:'));
    });

    test('starts on specified host and port', () {
      final server = MockServer.startOn('127.0.0.1', 18799);
      addTearDown(server.stop);
      expect(server.uri(), equals('http://127.0.0.1:18799'));
    });

    test('starts on port 0 assigns random port', () {
      final server = MockServer.startOn('127.0.0.1', 0);
      addTearDown(server.stop);
      final uri = server.uri();
      expect(uri, startsWith('http://127.0.0.1:'));
      final port = int.parse(uri.split(':').last);
      expect(port, greaterThan(0));
    });

    test('stop does not throw', () {
      final server = MockServer.start();
      addTearDown(server.stop);
      expect(server.stop, returnsNormally);
    });

    test('internal server error behavior is effective', () async {
      final server = MockServer.start();
      addTearDown(server.stop);
      final uri = server.uri();

      final behavior = MockBehavior.new_().withError(MockError.internalServerError);
      server.addMockBehavior(behavior);

      final client = HttpClient();
      addTearDown(client.close);
      final request = await client.getUrl(Uri.parse('$uri/repos/foo/bar'));
      final response = await request.close();

      expect(response.statusCode, equals(500));
      await response.drain();
    });

    test('rate limit exceeded behavior is effective', () async {
      final server = MockServer.start();
      addTearDown(server.stop);
      final uri = server.uri();

      final behavior = MockBehavior.new_().withError(MockError.rateLimitExceeded);
      server.addMockBehavior(behavior);

      final client = HttpClient();
      addTearDown(client.close);
      final request = await client.getUrl(Uri.parse('$uri/repos/foo/bar'));
      final response = await request.close();

      expect(response.statusCode, equals(403));
      await response.drain();
    });

    test('clearing behaviors restores normal response', () async {
      final server = MockServer.start();
      addTearDown(server.stop);
      final uri = server.uri();

      final behavior = MockBehavior.new_().withError(MockError.internalServerError);
      server.addMockBehavior(behavior);

      final client = HttpClient();
      addTearDown(client.close);

      var request = await client.getUrl(Uri.parse('$uri/repos/foo/bar'));
      var response = await request.close();
      expect(response.statusCode, equals(500));
      await response.drain();

      server.clearAllMockBehaviors();

      request = await client.getUrl(Uri.parse('$uri/repos/foo/bar'));
      response = await request.close();
      expect(response.statusCode, equals(404));
      await response.drain();
    });

    test('stop is idempotent', () {
      final server = MockServer.start();
      addTearDown(server.stop);
      server.stop();
      expect(server.stop, returnsNormally);
      expect(server.stop, returnsNormally);
    });

    test('throws MockServerError on invalid host', () {
      expect(
        () => MockServer.startOn('not an ip address', 0),
        throwsA(MockServerError.invalidHost),
      );
    });

    test('stops and allows re-start on same port', () {
      final server = MockServer.startOn('127.0.0.1', 18798);
      addTearDown(server.stop);
      final uri = server.uri();
      server.stop();

      final restarted = MockServer.startOn('127.0.0.1', 18798);
      addTearDown(restarted.stop);
      expect(restarted.uri(), equals(uri));
    });

    test('multiple start/stop cycles work', () {
      for (var i = 0; i < 3; i++) {
        final server = MockServer.start();
        addTearDown(server.stop);
        expect(server.uri(), startsWith('http://127.0.0.1:'));
        server.stop();
      }
    });

    test('can start on zero host', () {
      final server = MockServer.startOn('0.0.0.0', 0);
      addTearDown(server.stop);
      final uri = server.uri();
      expect(uri, startsWith('http://0.0.0.0:'));
    });

    test('startOn with port zero works on zero host', () {
      final server = MockServer.startOn('0.0.0.0', 0);
      addTearDown(server.stop);
      expect(server.uri(), startsWith('http://0.0.0.0:'));
    });

    test('server responds with 404 for unknown repository', () async {
      final server = MockServer.start();
      addTearDown(server.stop);
      final uri = server.uri();

      final client = HttpClient();
      addTearDown(client.close);
      final request = await client.getUrl(Uri.parse('$uri/repos/unknown/repo'));
      final response = await request.close();
      final body = await response.transform(utf8.decoder).join();

      expect(response.statusCode, equals(404));
      expect(body, contains('Not Found'));
    });

    test('server response body is valid JSON with message field', () async {
      final server = MockServer.start();
      addTearDown(server.stop);
      final uri = server.uri();

      final client = HttpClient();
      addTearDown(client.close);
      final request = await client.getUrl(Uri.parse('$uri/repos/foo/bar'));
      final response = await request.close();
      final body = await response.transform(utf8.decoder).join();

      final json = jsonDecode(body) as Map<String, dynamic>;
      expect(json['message'], equals('Not Found'));
      expect(json['documentation_url'], isA<String>());
    });

    test('can add and retrieve a repository', () async {
      final server = MockServer.start();
      addTearDown(server.stop);
      final uri = server.uri();

      final repo = Repository.new_('octocat', 'hello-world')
          .withDescription('A test repository')
          .withPrivate(true)
          .withStargazersCount(42)
          .withDefaultBranch('develop');

      server.addRepository(repo);

      final client = HttpClient();
      addTearDown(client.close);
      final request = await client.getUrl(Uri.parse('$uri/repos/octocat/hello-world'));
      final response = await request.close();
      final body = await response.transform(utf8.decoder).join();

      expect(response.statusCode, equals(200));
      final json = jsonDecode(body) as Map<String, dynamic>;
      expect(json['name'], equals('hello-world'));
      expect(json['owner']['login'], equals('octocat'));
      expect(json['description'], equals('A test repository'));
      expect(json['private'], isTrue);
      expect(json['stargazers_count'], equals(42));
      expect(json['default_branch'], equals('develop'));
    });

    test('can add and retrieve a repository with subscribers and network count', () async {
      final server = MockServer.start();
      addTearDown(server.stop);
      final uri = server.uri();

      final repo = Repository.new_('octocat', 'hello-world')
          .withSubscribersCount(123)
          .withNetworkCount(456);

      server.addRepository(repo);

      final client = HttpClient();
      addTearDown(client.close);
      final request = await client.getUrl(Uri.parse('$uri/repos/octocat/hello-world'));
      final response = await request.close();
      final body = await response.transform(utf8.decoder).join();

      expect(response.statusCode, equals(200));
      final json = jsonDecode(body) as Map<String, dynamic>;
      expect(json['subscribers_count'], equals(123));
      expect(json['network_count'], equals(456));
    });

    test('can add and retrieve a commit', () async {
      final server = MockServer.start();
      addTearDown(server.stop);
      final uri = server.uri();

      final commit = Commit.new_('octocat', 'hello-world')
          .withSha('abc123def456')
          .withMessage('A test commit')
          .withAuthorName('Test User')
          .withAuthorEmail('test@example.com');

      server.addCommit(commit);

      final client = HttpClient();
      addTearDown(client.close);
      final request = await client.getUrl(Uri.parse('$uri/repos/octocat/hello-world/commits/abc123def456'));
      final response = await request.close();
      final body = await response.transform(utf8.decoder).join();

      expect(response.statusCode, equals(200));
      final json = jsonDecode(body) as Map<String, dynamic>;
      expect(json['sha'], equals('abc123def456'));
      expect(json['commit']['message'], equals('A test commit'));
      expect(json['commit']['author']['name'], equals('Test User'));
      expect(json['commit']['author']['email'], equals('test@example.com'));
    });

    test('can add and retrieve an asset', () async {
      final server = MockServer.start();
      addTearDown(server.stop);
      final uri = server.uri();

      final content = utf8.encode('hello world');
      final asset = Asset.fromBytes('test.txt', content, 'text/plain');

      server.addAsset('octocat', 'hello-world', 'v1.0.0', asset);

      final client = HttpClient();
      addTearDown(client.close);
      final request = await client.getUrl(Uri.parse('$uri/octocat/hello-world/releases/download/v1.0.0/test.txt'));
      final response = await request.close();
      final body = await response.transform(utf8.decoder).join();

      expect(response.statusCode, equals(200));
      expect(body, equals('hello world'));
      expect(response.headers.contentType?.toString(), equals('text/plain'));
    });

    test('can add and retrieve a release with custom created_at', () async {
      final server = MockServer.start();
      addTearDown(server.stop);
      final uri = server.uri();

      const customTimestamp = '2023-12-25T12:00:00Z';
      final release = Release.new_('octocat', 'hello-world', 'v1.0.0')
          .withCreatedAt(customTimestamp);

      server.addRelease(release);

      final client = HttpClient();
      addTearDown(client.close);
      final request = await client.getUrl(Uri.parse('$uri/repos/octocat/hello-world/releases/tags/v1.0.0'));
      final response = await request.close();
      final body = await response.transform(utf8.decoder).join();

      expect(response.statusCode, equals(200));
      final json = jsonDecode(body) as Map<String, dynamic>;
      expect(json['tag_name'], equals('v1.0.0'));
      expect(json['created_at'], equals(customTimestamp));
      expect(json['published_at'], equals(customTimestamp));
    });
  });
}
