#include <check.h>
#include <stdlib.h>
#include <string.h>

#include "github_mock_api/MockServer.h"
#include "github_mock_api/MockBehavior.h"
#include "github_mock_api/diplomat_runtime.h"

#define URI_PREFIX "http://127.0.0.1:"

#define DS(s) ((DiplomatStringView){ .data = (s), .len = sizeof(s) - 1 })

static void assert_uri_valid(MockServer *server) {
    DiplomatWrite *write = diplomat_buffer_write_create(64);
    ck_assert_ptr_nonnull(write);

    MockServer_uri(server, write);

    const char *uri = (const char *)diplomat_buffer_write_get_bytes(write);
    size_t len = diplomat_buffer_write_len(write);

    ck_assert_uint_gt(len, 0);
    ck_assert(strncmp(uri, URI_PREFIX, strlen(URI_PREFIX)) == 0);

    diplomat_buffer_write_destroy(write);
}

START_TEST(test_start_stop) {
    MockServer_start_result started = MockServer_start();
    ck_assert(started.is_ok);

    MockServer *server = started.ok;
    ck_assert_ptr_nonnull(server);

    assert_uri_valid(server);

    MockServer_stop_result stopped = MockServer_stop(server);
    ck_assert(stopped.is_ok);

    MockServer_destroy(server);
}
END_TEST

START_TEST(test_start_on_specific_port) {
    MockServer_start_on_result started = MockServer_start_on(DS("127.0.0.1"), 19876);
    ck_assert(started.is_ok);

    MockServer *server = started.ok;
    assert_uri_valid(server);

    MockServer_stop_result stopped = MockServer_stop(server);
    ck_assert(stopped.is_ok);

    MockServer_destroy(server);
}
END_TEST

START_TEST(test_start_on_random_port) {
    MockServer_start_on_result started = MockServer_start_on(DS("127.0.0.1"), 0);
    ck_assert(started.is_ok);

    MockServer *server = started.ok;
    assert_uri_valid(server);

    MockServer_stop_result stopped = MockServer_stop(server);
    ck_assert(stopped.is_ok);

    MockServer_destroy(server);
}
END_TEST

START_TEST(test_two_servers) {
    MockServer_start_result r1 = MockServer_start();
    ck_assert(r1.is_ok);

    MockServer_start_result r2 = MockServer_start();
    ck_assert(r2.is_ok);

    MockServer *s1 = r1.ok;
    MockServer *s2 = r2.ok;

    ck_assert_ptr_nonnull(s1);
    ck_assert_ptr_nonnull(s2);

    assert_uri_valid(s1);
    assert_uri_valid(s2);

    MockServer_stop_result stop1 = MockServer_stop(s1);
    ck_assert(stop1.is_ok);

    MockServer_stop_result stop2 = MockServer_stop(s2);
    ck_assert(stop2.is_ok);

    MockServer_destroy(s1);
    MockServer_destroy(s2);
}
END_TEST

START_TEST(test_stop_idempotency) {
    MockServer_start_result started = MockServer_start();
    ck_assert(started.is_ok);

    MockServer *server = started.ok;

    MockServer_stop_result stop1 = MockServer_stop(server);
    ck_assert(stop1.is_ok);

    MockServer_stop_result stop2 = MockServer_stop(server);
    ck_assert(stop2.is_ok);

    MockServer_destroy(server);
}
END_TEST

START_TEST(test_invalid_host) {
    MockServer_start_on_result started = MockServer_start_on(DS("invalid-host-that-does-not-exist!!!"), 0);
    ck_assert(!started.is_ok);
    ck_assert_int_eq((int)started.err, (int)MockServerError_InvalidHost);
}
END_TEST

START_TEST(test_mock_behavior_conflict) {
    MockServer_start_result started = MockServer_start();
    ck_assert(started.is_ok);
    MockServer *server = started.ok;

    MockBehavior *b1_builder = MockBehavior_new();
    MockBehavior *b1 = MockBehavior_with_error(b1_builder, MockError_InternalServerError);
    MockBehavior_destroy(b1_builder);

    MockBehavior *b2_builder = MockBehavior_new();
    MockBehavior *b2 = MockBehavior_with_error(b2_builder, MockError_RateLimitExceeded);
    MockBehavior_destroy(b2_builder);

    MockServer_add_mock_behavior_result res1 = MockServer_add_mock_behavior(server, b1);
    ck_assert(res1.is_ok);

    MockServer_add_mock_behavior_result res2 = MockServer_add_mock_behavior(server, b2);
    ck_assert(!res2.is_ok);
    ck_assert_int_eq((int)res2.err, (int)MockServerError_Conflict);

    MockServer_clear_all_mock_behaviors_result res3 = MockServer_clear_all_mock_behaviors(server);
    ck_assert(res3.is_ok);

    // After clearing, we should be able to add b2
    MockServer_add_mock_behavior_result res4 = MockServer_add_mock_behavior(server, b2);
    ck_assert(res4.is_ok);

    MockBehavior_destroy(b1);
    MockBehavior_destroy(b2);
    MockServer_stop_result stopped = MockServer_stop(server);
    ck_assert(stopped.is_ok);
    MockServer_destroy(server);
}
END_TEST

START_TEST(test_mock_behavior_immutable_builder) {
    MockBehavior *b1 = MockBehavior_new();
    MockBehavior *b2 = MockBehavior_with_error(b1, MockError_InternalServerError);

    // b1 should still have no error, b2 should have error
    // We can't easily inspect inner state from C, but we can verify they are different objects
    ck_assert_ptr_ne(b1, b2);

    MockServer_start_result started = MockServer_start();
    ck_assert(started.is_ok);
    MockServer *server = started.ok;

    // Adding b1 (no error) should not cause 500
    MockServer_add_mock_behavior_result res1 = MockServer_add_mock_behavior(server, b1);
    ck_assert(res1.is_ok);

    // Adding b2 (InternalServerError) should succeed but subsequent add should conflict
    MockServer_clear_all_mock_behaviors(server);
    MockServer_add_mock_behavior_result res2 = MockServer_add_mock_behavior(server, b2);
    ck_assert(res2.is_ok);

    MockBehavior *b3 = MockBehavior_with_error(b2, MockError_RateLimitExceeded);
    ck_assert_ptr_ne(b2, b3);

    MockServer_add_mock_behavior_result res3 = MockServer_add_mock_behavior(server, b3);
    ck_assert(!res3.is_ok);
    ck_assert_int_eq((int)res3.err, (int)MockServerError_Conflict);

    MockBehavior_destroy(b1);
    MockBehavior_destroy(b2);
    MockBehavior_destroy(b3);
    MockServer_stop(server);
    MockServer_destroy(server);
}
END_TEST


START_TEST(test_start_after_stop) {
    MockServer_start_result r1 = MockServer_start();
    ck_assert(r1.is_ok);

    MockServer *s1 = r1.ok;
    assert_uri_valid(s1);

    MockServer_stop_result stop1 = MockServer_stop(s1);
    ck_assert(stop1.is_ok);

    MockServer_destroy(s1);

    MockServer_start_result r2 = MockServer_start();
    ck_assert(r2.is_ok);

    MockServer *s2 = r2.ok;
    assert_uri_valid(s2);

    MockServer_stop_result stop2 = MockServer_stop(s2);
    ck_assert(stop2.is_ok);

    MockServer_destroy(s2);
}
END_TEST

Suite *mock_server_suite(void) {
    Suite *s = suite_create("MockServer");
    TCase *tc_core = tcase_create("Core");

    tcase_add_test(tc_core, test_start_stop);
    tcase_add_test(tc_core, test_start_on_specific_port);
    tcase_add_test(tc_core, test_start_on_random_port);
    tcase_add_test(tc_core, test_two_servers);
    tcase_add_test(tc_core, test_stop_idempotency);
    tcase_add_test(tc_core, test_invalid_host);
    tcase_add_test(tc_core, test_mock_behavior_conflict);
    tcase_add_test(tc_core, test_mock_behavior_immutable_builder);
    tcase_add_test(tc_core, test_start_after_stop);

    suite_add_tcase(s, tc_core);
    return s;
}

int main(void) {
    Suite *s = mock_server_suite();
    SRunner *sr = srunner_create(s);
    srunner_run_all(sr, CK_NORMAL);
    int failed = srunner_ntests_failed(sr);
    srunner_free(sr);
    return failed;
}
