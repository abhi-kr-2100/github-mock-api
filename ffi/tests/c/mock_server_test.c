#include <check.h>
#include <stdlib.h>
#include <string.h>

#include "github_mock_api/MockServer.h"
#include "github_mock_api/Repository.h"
#include "github_mock_api/Release.h"
#include "github_mock_api/Commit.h"
#include "github_mock_api/Asset.h"
#include "github_mock_api/MockBehavior.h"
#include "github_mock_api/diplomat_runtime.h"

#define URI_PREFIX "http://127.0.0.1:"

#define DS(s) ((DiplomatStringView){ .data = (s), .len = sizeof(s) - 1 })
#define DSD(s) ((DiplomatStringView){ .data = (s), .len = strlen(s) })

static char* get_data_dir() {
    static char path[1024];
    const char* file = __FILE__;
    const char* last_slash = strrchr(file, '/');
    if (!last_slash) last_slash = strrchr(file, '\\');

    if (last_slash) {
        size_t len = last_slash - file;
        strncpy(path, file, len);
        path[len] = '\0';
        strcat(path, "/../../../testing/data");
    } else {
        strcpy(path, "testing/data");
    }
    return path;
}

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

START_TEST(test_data_registration) {
    MockServer_start_result started = MockServer_start();
    ck_assert(started.is_ok);
    MockServer *server = started.ok;

    // Repository
    Repository *repo = Repository_new(DS("octocat"), DS("hello-world"));
    Repository *repo2 = Repository_description(repo, DS("A great repo"));
    ck_assert_ptr_ne(repo, repo2);

    MockServer_add_repository_result res_repo = MockServer_add_repository(server, repo2);
    ck_assert(res_repo.is_ok);

    // Release
    Release *rel = Release_new(DS("octocat"), DS("hello-world"), DS("v1.0.0"));
    Release *rel2 = Release_name(rel, DS("v1.0.0 Release"));
    ck_assert_ptr_ne(rel, rel2);

    MockServer_add_release_result res_rel = MockServer_add_release(server, DS("octocat"), DS("hello-world"), rel2);
    ck_assert(res_rel.is_ok);

    // Commit
    Commit *commit = Commit_new(DS("octocat"), DS("hello-world"));
    Commit *commit2 = Commit_sha(commit, DS("abc123def"));
    ck_assert_ptr_ne(commit, commit2);

    MockServer_add_commit_result res_commit = MockServer_add_commit(server, DS("octocat"), DS("hello-world"), commit2);
    ck_assert(res_commit.is_ok);

    // Asset
    uint8_t data[] = {'h', 'e', 'l', 'l', 'o'};
    Asset *asset = Asset_from_bytes(DS("test.txt"), data, sizeof(data), DS("text/plain"));
    MockServer_add_asset_result res_asset = MockServer_add_asset(server, DS("octocat"), DS("hello-world"), DS("v1.0.0"), asset);
    ck_assert(res_asset.is_ok);

    // Mock Behavior
    MockBehavior *behavior = MockBehavior_new_error(MockError_InternalServerError);
    MockServer_add_mock_behavior_result res_behavior = MockServer_add_mock_behavior(server, behavior);
    ck_assert(res_behavior.is_ok);

    MockServer_clear_all_mock_behaviors_result res_clear = MockServer_clear_all_mock_behaviors(server);
    ck_assert(res_clear.is_ok);

    Repository_destroy(repo);
    Repository_destroy(repo2);
    Release_destroy(rel);
    Release_destroy(rel2);
    Commit_destroy(commit);
    Commit_destroy(commit2);
    Asset_destroy(asset);
    MockBehavior_destroy(behavior);
    MockServer_stop(server);
    MockServer_destroy(server);
}
END_TEST

START_TEST(test_load_from_file) {
    MockServer_start_result started = MockServer_start();
    ck_assert(started.is_ok);
    MockServer *server = started.ok;

    char path[1024];
    char* data_dir = get_data_dir();

    // Repositories
    sprintf(path, "%s/repositories.json", data_dir);
    MockServer_add_repositories_from_file_result res1 = MockServer_add_repositories_from_file(server, DSD(path));
    ck_assert(res1.is_ok);

    // Releases
    sprintf(path, "%s/releases.json", data_dir);
    MockServer_add_releases_from_file_result res2 = MockServer_add_releases_from_file(server, DSD(path), DS("CleverRaven"), DS("Cataclysm-DDA"));
    ck_assert(res2.is_ok);

    // Commits
    sprintf(path, "%s/commits.json", data_dir);
    MockServer_add_commits_from_file_result res3 = MockServer_add_commits_from_file(server, DSD(path), DS("karpathy"), DS("arxiv-sanity-lite"));
    ck_assert(res3.is_ok);

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
    tcase_add_test(tc_core, test_start_after_stop);
    tcase_add_test(tc_core, test_data_registration);
    tcase_add_test(tc_core, test_load_from_file);

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
