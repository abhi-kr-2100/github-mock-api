//! Smoke-test the generated C API by compiling and running `tests/c/mock_server.c`.

use std::fs::create_dir_all;
use std::path::{PathBuf, Path};
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

    #[error("CARGO environment variable not set")]
    CargoEnvMissing,

    #[error("spawn cargo build for cdylib")]
    SpawnCargoBuild,

    #[error("cargo build for cdylib failed")]
    CargoBuildFailed,

    #[error("run C API smoke test binary")]
    RunSmokeTest,
}

#[test]
fn c_mock_server_smoke_test() -> Result<(), TestError> {
    build_cdylib()?;

    let out_dir = target_dir()?.join("c-api-tests");
    create_dir_all(&out_dir).map_err(|_| TestError::CreateOutputDir)?;

    let cc = which_cc().ok_or(TestError::NoCCompiler)?;
    let binary = out_dir.join("mock_server");

    compile_mock_server(&cc, &binary)?;

    let run_status = Command::new(&binary)
        .status()
        .map_err(|_| TestError::RunSmokeTest)?;
    assert!(run_status.success(), "C API smoke test exited with failure");
    
    Ok(())
}

fn workspace_root() -> Result<PathBuf, TestError> {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .ok_or(TestError::WorkspaceRoot)
        .map(|p| p.to_path_buf())
}

fn target_dir() -> Result<PathBuf, TestError> {
    Ok(workspace_root()?.join("target"))
}

fn profile_dir() -> Result<PathBuf, TestError> {
    let profile = if cfg!(debug_assertions) { "debug" } else { "release" };
    Ok(target_dir()?.join(profile))
}

fn build_cdylib() -> Result<(), TestError> {
    let target = target_dir()?;
    let cargo = std::env::var("CARGO").map_err(|_| TestError::CargoEnvMissing)?;
    let profile = if cfg!(debug_assertions) { "debug" } else { "release" };
    let mut cmd = Command::new(cargo);
    cmd.arg("build")
        .arg("-p")
        .arg("github-mock-api-ffi")
        .arg("--lib")
        .arg("--target-dir")
        .arg(&target);
    if profile != "debug" {
        cmd.arg("--profile").arg(profile);
    }
    let status = cmd.status().map_err(|_| TestError::SpawnCargoBuild)?;
    if !status.success() {
        return Err(TestError::CargoBuildFailed);
    }
    Ok(())
}

fn rpath_args(lib_dir: &Path) -> Vec<String> {
    let mut args = Vec::new();
    let rpath = lib_dir.display().to_string();
    if cfg!(target_os = "linux") || cfg!(target_os = "macos") {
        args.push(format!("-Wl,-rpath,{rpath}"));
    }
    args
}

fn compile_mock_server(cc: &str, binary: &PathBuf) -> Result<(), TestError> {
    let source = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/c/mock_server.c");
    let include = workspace_root()?.join("include");
    let lib_dir = profile_dir()?;

    let mut compile = Command::new(cc);
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

    for arg in rpath_args(&lib_dir) {
        compile.arg(arg);
    }
    compile.arg("-o").arg(binary);

    if cfg!(target_os = "linux") {
        compile.arg("-pthread");
    }

    let compile_status = compile.status().map_err(|_| TestError::RunCCompiler)?;
    assert!(
        compile_status.success(),
        "failed to compile C API smoke test"
    );

    Ok(())
}

fn lib_name() -> &'static str {
    "github_mock_api_ffi"
}

fn which_cc() -> Option<String> {
    for candidate in ["cc"] {
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
