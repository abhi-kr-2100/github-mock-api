#include <assert.h>
#include <stdio.h>
#include <string.h>

#include "github_mock_api/MockServer.h"
#include "github_mock_api/diplomat_runtime.h"

int main(void) {
    MockServer_start_result started = MockServer_start();
    if (!started.is_ok) {
        fprintf(stderr, "MockServer_start failed: %d\n", (int)started.err);
        return 1;
    }

    MockServer *server = started.ok;

    DiplomatWrite *write = diplomat_buffer_write_create(64);
    if (write == NULL) {
        fprintf(stderr, "diplomat_buffer_write_create failed\n");
        MockServer_destroy(server);
        return 1;
    }

    MockServer_uri(server, write);

    const char *uri = (const char *)diplomat_buffer_write_get_bytes(write);
    size_t len = diplomat_buffer_write_len(write);
    if (len == 0 || strncmp(uri, "http://127.0.0.1:", 17) != 0) {
        fprintf(stderr, "unexpected uri: %.*s\n", (int)len, uri);
        diplomat_buffer_write_destroy(write);
        MockServer_destroy(server);
        return 1;
    }

    diplomat_buffer_write_destroy(write);

    MockServer_stop_result stopped = MockServer_stop(server);
    if (!stopped.is_ok) {
        fprintf(stderr, "MockServer_stop failed: %d\n", (int)stopped.err);
        MockServer_destroy(server);
        return 1;
    }

    MockServer_destroy(server);
    return 0;
}
