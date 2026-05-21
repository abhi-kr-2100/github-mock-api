//! Smoke-test the generated C API by compiling and running `tests/c/mock_server.c`.

use std::path::PathBuf;
use std::process::Command;
use thiserror::Error;

#[derive(Error, Debug)]
enum TestError {
    #[error("ffi crate lives in the workspace root")]
    WorkspaceRoot,

    #[error("C compiler required for C API tests (cc or gcc)")]
    NoCCompiler,

    #[error("create c-api-tests output dir")]
    CreateOutputDir,

    #[error("run C compiler")]
    RunCCompiler,

    #[error("run C API smoke test binary")]
    RunSmokeTest,

    #[error("failed to parse target path")]
    ParseTargetPath,
}

fn workspace_root() -> Result<PathBuf, TestError> {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .ok_or(TestError::WorkspaceRoot)
        .map(|p| p.to_path_buf())
}

fn target_dir() -> Result<PathBuf, TestError> {
    match std::env::var("CARGO_TARGET_DIR") {
        Ok(val) => Ok(PathBuf::from(val)),
        Err(_) => {
            workspace_root()?
                .join("target")
                .into_os_string()
                .into_string()
                .map_err(|_| TestError::ParseTargetPath)
                .map(PathBuf::from)
        }
    }
}

fn profile_dir() -> Result<PathBuf, TestError> {
    let profile = std::env::var("CARGO_BUILD_PROFILE")
        .or_else(|_| std::env::var("PROFILE"))
        .unwrap_or_else(|_| "debug".to_string());
    Ok(target_dir()?.join(profile))
}

#[test]
fn c_mock_server_smoke_test() -> Result<(), TestError> {
    let cc = which_cc().ok_or(TestError::NoCCompiler)?;
    let root = workspace_root()?;
    let lib_dir = profile_dir()?;
    let out_dir = target_dir()?.join("c-api-tests");
    std::fs::create_dir_all(&out_dir).map_err(|_| TestError::CreateOutputDir)?;

    let source = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/c/mock_server.c");
    let binary = out_dir.join("mock_server");
    let include = root.join("include");

    let mut compile = Command::new(&cc);
    compile
        .arg("-std=c11")
        .arg("-Wall")
        .arg("-Wextra")
        .arg("-I")
        .arg(&include)
        .arg(&source)
        .arg("-L")
        .arg(&lib_dir)
        .arg("-l")
        .arg(lib_name());

    add_rpath(&mut compile, &lib_dir);
    compile.arg("-o").arg(&binary);

    if cfg!(target_os = "linux") {
        compile.arg("-pthread");
    }

    let compile_status = compile.status().map_err(|_| TestError::RunCCompiler)?;
    assert!(
        compile_status.success(),
        "failed to compile C API smoke test"
    );

    let run_status = Command::new(&binary)
        .status()
        .map_err(|_| TestError::RunSmokeTest)?;
    assert!(run_status.success(), "C API smoke test exited with failure");
    
    Ok(())
}

fn lib_name() -> &'static str {
    "github_mock_api_ffi"
}

fn add_rpath(compile: &mut Command, lib_dir: &PathBuf) {
    let rpath = lib_dir.display().to_string();
    if cfg!(target_os = "linux") {
        compile.arg(format!("-Wl,-rpath,{rpath}"));
    } else if cfg!(target_os = "macos") {
        compile.arg(format!("-Wl,-rpath,{rpath}"));
    }
}

fn which_cc() -> Option<String> {
    for candidate in ["cc", "gcc", "clang"] {
        if let Ok(output) = Command::new(candidate)
            .arg("--version")
            .output() {
                if output.status.success() {
                    return Some(candidate.to_string());
                }
            }
    }
    None
}
