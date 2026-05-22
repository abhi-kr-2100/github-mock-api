import 'dart:io';

import '../../../bindings/dart/lib.g.dart';

void main() {
  try {
    final server = MockServer.start();
    final uri = server.uri();
    if (!uri.startsWith('http://127.0.0.1:')) {
      stderr.writeln('unexpected uri: $uri');
      server.stop();
      exit(1);
    }
    server.stop();
  } catch (e) {
    stderr.writeln('MockServer test failed: $e');
    exit(1);
  }
}
