#include <check.h>
#include <stdlib.h>
#include <string.h>

#include "github_mock_api/MockServer.h"
#include "github_mock_api/MockBehavior.h"
#include "github_mock_api/Repository.h"
#include "github_mock_api/Commit.h"
#include "github_mock_api/Asset.h"
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
    MockServer_clear_all_mock_behaviors_result clear_res = MockServer_clear_all_mock_behaviors(server);
    ck_assert(clear_res.is_ok);
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
    MockServer_stop_result stopped2 = MockServer_stop(server);
    ck_assert(stopped2.is_ok);
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

START_TEST(test_add_repository) {
    MockServer_start_result started = MockServer_start();
    ck_assert(started.is_ok);
    MockServer *server = started.ok;

    Repository *repo_builder = Repository_new(DS("octocat"), DS("hello-world"));
    Repository *repo = Repository_with_description(repo_builder, DS("A test repository"));
    Repository_destroy(repo_builder);

    Repository *repo2 = Repository_with_private(repo, true);
    Repository *repo3 = Repository_with_stargazers_count(repo2, 42);
    Repository *repo4 = Repository_with_default_branch(repo3, DS("develop"));

    MockServer_add_repository_result res = MockServer_add_repository(server, repo4);
    ck_assert(res.is_ok);

    Repository_destroy(repo);
    Repository_destroy(repo2);
    Repository_destroy(repo3);
    Repository_destroy(repo4);

    MockServer_stop_result stopped = MockServer_stop(server);
    ck_assert(stopped.is_ok);
    MockServer_destroy(server);
}
END_TEST

START_TEST(test_repository_e2e) {
    MockServer_start_result started = MockServer_start();
    ck_assert(started.is_ok);
    MockServer *server = started.ok;

    Repository *repo = Repository_new(DS("octocat"), DS("hello-world"));
    MockServer_add_repository_result res = MockServer_add_repository(server, repo);
    ck_assert(res.is_ok);

    DiplomatWrite *write = diplomat_buffer_write_create(64);
    MockServer_uri(server, write);
    const char *base_uri = (const char *)diplomat_buffer_write_get_bytes(write);
    size_t base_uri_len = diplomat_buffer_write_len(write);

    char curl_cmd[256];
    snprintf(curl_cmd, sizeof(curl_cmd), "curl -s -f %.*s/repos/octocat/hello-world > /dev/null", (int)base_uri_len, base_uri);

    int curl_res = system(curl_cmd);
    ck_assert_int_eq(curl_res, 0);

    diplomat_buffer_write_destroy(write);
    Repository_destroy(repo);
    MockServer_stop_result stopped = MockServer_stop(server);
    ck_assert(stopped.is_ok);
    MockServer_destroy(server);
}
END_TEST

START_TEST(test_add_commit) {
    MockServer_start_result started = MockServer_start();
    ck_assert(started.is_ok);
    MockServer *server = started.ok;

    Commit *commit_builder = Commit_new(DS("octocat"), DS("hello-world"));
    Commit *commit = Commit_with_message(commit_builder, DS("A test commit"));
    Commit_destroy(commit_builder);

    Commit *commit2 = Commit_with_sha(commit, DS("abc123def456"));
    Commit *commit3 = Commit_with_author_name(commit2, DS("Test User"));
    Commit *commit4 = Commit_with_author_email(commit3, DS("test@example.com"));

    MockServer_add_commit_result res = MockServer_add_commit(server, commit4);
    ck_assert(res.is_ok);

    Commit_destroy(commit);
    Commit_destroy(commit2);
    Commit_destroy(commit3);
    Commit_destroy(commit4);

    MockServer_stop_result stopped = MockServer_stop(server);
    ck_assert(stopped.is_ok);
    MockServer_destroy(server);
}
END_TEST

START_TEST(test_commit_e2e) {
    MockServer_start_result started = MockServer_start();
    ck_assert(started.is_ok);
    MockServer *server = started.ok;

    Commit *commit_builder = Commit_new(DS("octocat"), DS("hello-world"));
    Commit *commit = Commit_with_sha(commit_builder, DS("abc123def456"));
    Commit_destroy(commit_builder);

    MockServer_add_commit_result res = MockServer_add_commit(server, commit);
    ck_assert(res.is_ok);

    DiplomatWrite *write = diplomat_buffer_write_create(64);
    MockServer_uri(server, write);
    const char *base_uri = (const char *)diplomat_buffer_write_get_bytes(write);
    size_t base_uri_len = diplomat_buffer_write_len(write);

    char curl_cmd[256];
    snprintf(curl_cmd, sizeof(curl_cmd), "curl -s -f %.*s/repos/octocat/hello-world/commits/abc123def456 > /dev/null", (int)base_uri_len, base_uri);

    int curl_res = system(curl_cmd);
    ck_assert_int_eq(curl_res, 0);

    diplomat_buffer_write_destroy(write);
    Commit_destroy(commit);
    MockServer_stop_result stopped = MockServer_stop(server);
    ck_assert(stopped.is_ok);
    MockServer_destroy(server);
}
END_TEST

START_TEST(test_asset_e2e) {
    MockServer_start_result started = MockServer_start();
    ck_assert(started.is_ok);
    MockServer *server = started.ok;

    const char content[] = "hello world";
    DiplomatU8View bytes = { .data = (const uint8_t*)content, .len = sizeof(content) - 1 };
    Asset *asset = Asset_from_bytes(DS("test.txt"), bytes, DS("text/plain"));

    MockServer_add_asset_result res = MockServer_add_asset(server, DS("octocat"), DS("hello-world"), DS("v1.0.0"), asset);
    ck_assert(res.is_ok);

    DiplomatWrite *write = diplomat_buffer_write_create(64);
    MockServer_uri(server, write);
    const char *base_uri = (const char *)diplomat_buffer_write_get_bytes(write);
    size_t base_uri_len = diplomat_buffer_write_len(write);

    char curl_cmd[256];
    snprintf(curl_cmd, sizeof(curl_cmd), "curl -s -f %.*s/octocat/hello-world/releases/download/v1.0.0/test.txt | grep -q \"hello world\"", (int)base_uri_len, base_uri);

    int curl_res = system(curl_cmd);
    ck_assert_int_eq(curl_res, 0);

    diplomat_buffer_write_destroy(write);
    Asset_destroy(asset);
    MockServer_stop_result stopped = MockServer_stop(server);
    ck_assert(stopped.is_ok);
    MockServer_destroy(server);
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
    tcase_add_test(tc_core, test_add_repository);
    tcase_add_test(tc_core, test_repository_e2e);
    tcase_add_test(tc_core, test_add_commit);
    tcase_add_test(tc_core, test_commit_e2e);
    tcase_add_test(tc_core, test_asset_e2e);

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
