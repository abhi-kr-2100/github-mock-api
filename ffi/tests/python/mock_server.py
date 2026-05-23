import sys

from github_mock_api import MockServer


def main() -> None:
    try:
        server = MockServer.start()
        uri = server.uri()
        if not uri.startswith("http://127.0.0.1:"):
            print(f"unexpected uri: {uri}", file=sys.stderr)
            server.stop()
            sys.exit(1)

        server.stop()
    except Exception as e:
        print(f"MockServer test failed: {e}", file=sys.stderr)
        sys.exit(1)


if __name__ == "__main__":
    main()
