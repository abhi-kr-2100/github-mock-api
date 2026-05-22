use std::process::Command;

use crate::common::{build_cdylib, mock_api_lib_path, preload_env_vars, workspace_root, TestError};

fn which_dart() -> Option<String> {
    if let Ok(output) = Command::new("dart").arg("--version").output() {
        if output.status.success() {
            return Some("dart".to_string());
        }
    }
    None
}

#[test]
fn dart_mock_server_smoke_test() -> Result<(), TestError> {
    build_cdylib()?;

    let root = workspace_root()?;
    let test_dir = root.join("ffi/tests/dart");
    let lib_path = mock_api_lib_path()?;

    let dart = which_dart().ok_or(TestError::NoDartRuntime)?;

    // Fetch Dart package dependencies (package:ffi, package:meta)
    let pub_get = Command::new(&dart)
        .arg("pub")
        .arg("get")
        .current_dir(&test_dir)
        .status()
        .map_err(|_| TestError::RunDartTest)?;
    assert!(
        pub_get.success(),
        "dart pub get failed in {}",
        test_dir.display()
    );

    // Run the smoke test with the library preloaded so @ffi.Native symbols resolve
    let mut cmd = Command::new(&dart);
    cmd.arg("run")
        .arg("mock_server.dart")
        .current_dir(&test_dir);
    for (key, val) in preload_env_vars(&lib_path) {
        cmd.env(key, val);
    }

    let status = cmd.status().map_err(|_| TestError::RunDartTest)?;
    assert!(
        status.success(),
        "Dart API smoke test exited with failure"
    );

    Ok(())
}
