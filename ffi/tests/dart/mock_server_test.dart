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

    test('registers repository and responds with 200', () async {
      final server = MockServer.start();
      addTearDown(server.stop);
      final uri = server.uri();

      final repo = Repository.new_('octocat', 'hello-world');
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
    });

    test('applies mock behavior and returns 500', () async {
      final server = MockServer.start();
      addTearDown(server.stop);
      final uri = server.uri();

      final behavior = MockBehavior.newError(MockError.internalServerError);
      server.addMockBehavior(behavior);

      final client = HttpClient();
      addTearDown(client.close);
      final request = await client.getUrl(Uri.parse('$uri/repos/any/repo'));
      final response = await request.close();

      expect(response.statusCode, equals(500));

      server.clearAllMockBehaviors();
      final request2 = await client.getUrl(Uri.parse('$uri/repos/any/repo'));
      final response2 = await request2.close();
      expect(response2.statusCode, equals(404));
    });
  });
}
