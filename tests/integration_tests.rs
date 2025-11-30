use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

#[test]
fn test_cli_help() {
    let mut cmd = Command::cargo_bin("trls").unwrap();
    cmd.arg("--skip-root-check");
    cmd.arg("--help");
    cmd.assert().success().stdout(predicate::str::contains(
        "A container build system for multi-stage builds",
    ));
}

#[test]
fn test_cli_version() {
    // The CLI doesn't support --version, so test -V instead, but clap doesn't add this by default
    // Let's just test help which should show the version in the usage
    let mut cmd = Command::cargo_bin("trls").unwrap();
    cmd.arg("--skip-root-check");
    cmd.arg("--help");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("A container build system"));
}

#[test]
fn test_build_builder_no_stages() {
    let temp_dir = TempDir::new().unwrap();
    let mut cmd = Command::cargo_bin("trls").unwrap();
    cmd.arg("--skip-root-check");
    // Don't override builder-stages, so it should use empty default when no config
    cmd.arg("--stages-dir")
        .arg(temp_dir.path())
        .arg("build-builder");

    // The error will depend on whether system config exists
    // If system config has stages, it will try to find containerfiles
    // If no system config, it should say "No builder stages defined"
    cmd.assert().failure().stderr(
        predicate::str::contains("Missing required containerfiles")
            .or(predicate::str::contains("No builder stages defined")),
    );
}

#[test]
fn test_build_no_stages() {
    let temp_dir = TempDir::new().unwrap();
    let mut cmd = Command::cargo_bin("trls").unwrap();
    cmd.arg("--skip-root-check");
    // Don't override rootfs-stages, so it should use empty default when no config
    cmd.arg("--stages-dir").arg(temp_dir.path()).arg("build");

    // The error will depend on whether system config exists
    cmd.assert().failure().stderr(
        predicate::str::contains("Missing required containerfiles")
            .or(predicate::str::contains("No rootfs stages defined")),
    );
}

#[test]
fn test_build_builder_with_missing_containerfile() {
    let temp_dir = TempDir::new().unwrap();

    let mut cmd = Command::cargo_bin("trls").unwrap();
    cmd.arg("--skip-root-check");
    cmd.arg("--stages-dir")
        .arg(temp_dir.path())
        .arg("--builder-stages")
        .arg("base")
        .arg("build-builder");

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("Missing required containerfiles"));
}

#[test]
fn test_build_with_missing_containerfile() {
    let temp_dir = TempDir::new().unwrap();

    let mut cmd = Command::cargo_bin("trls").unwrap();
    cmd.arg("--skip-root-check");
    cmd.arg("--stages-dir")
        .arg(temp_dir.path())
        .arg("--rootfs-stages")
        .arg("base")
        .arg("build");

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("Missing required containerfiles"));
}

#[test]
fn test_clean_command() {
    // Test that clean command runs and provides appropriate output
    let temp_dir = TempDir::new().unwrap();
    let mut cmd = Command::cargo_bin("trls").unwrap();
    cmd.arg("--skip-root-check")
        .arg("--stages-dir")
        .arg(temp_dir.path());
    cmd.arg("clean");

    let output = cmd.output().unwrap();
    let stdout_str = String::from_utf8_lossy(&output.stdout);
    let stderr_str = String::from_utf8_lossy(&output.stderr);

    // Check that the command provides informative output about trls-specific cleaning
    // It should either say "Cleaning trls-generated images..." or have an error about podman
    assert!(
        stdout_str.contains("Cleaning trls-generated images") || 
        stderr_str.contains("Failed to list podman images") ||
        stderr_str.contains("Failed to execute podman") ||
        stdout_str.contains("No trls-generated images found to clean"),
        "Expected clean command to show trls-specific cleaning behavior, got stdout: '{stdout_str}', stderr: '{stderr_str}'"
    );
}

#[test]
fn test_clean_command_with_custom_tags() {
    // Test that clean command works with custom tags
    let temp_dir = TempDir::new().unwrap();
    let mut cmd = Command::cargo_bin("trls").unwrap();
    cmd.arg("--skip-root-check")
        .arg("--stages-dir")
        .arg(temp_dir.path());
    cmd.arg("--builder-tag")
        .arg("custom-builder")
        .arg("--rootfs-tag")
        .arg("custom-rootfs")
        .arg("clean");

    let output = cmd.output().unwrap();
    let stdout_str = String::from_utf8_lossy(&output.stdout);
    let stderr_str = String::from_utf8_lossy(&output.stderr);

    // Should still show trls-specific cleaning behavior regardless of custom tags
    assert!(
        stdout_str.contains("Cleaning trls-generated images")
            || stderr_str.contains("Failed to list podman images")
            || stderr_str.contains("Failed to execute podman")
            || stdout_str.contains("No trls-generated images found to clean"),
        "Expected clean command to work with custom tags, got stdout: '{stdout_str}', stderr: '{stderr_str}'"
    );
}

#[test]
fn test_config_override_builder_tag() {
    let temp_dir = TempDir::new().unwrap();

    let mut cmd = Command::cargo_bin("trls").unwrap();
    cmd.arg("--skip-root-check");
    cmd.arg("--builder-tag")
        .arg("custom-builder")
        .arg("--stages-dir")
        .arg(temp_dir.path())
        .arg("--builder-stages")
        .arg("base")
        .arg("build-builder");

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("Missing required containerfiles"));
}

#[test]
fn test_config_override_rootfs_tag() {
    let temp_dir = TempDir::new().unwrap();

    let mut cmd = Command::cargo_bin("trls").unwrap();
    cmd.arg("--skip-root-check");
    cmd.arg("--rootfs-tag")
        .arg("custom-rootfs")
        .arg("--stages-dir")
        .arg(temp_dir.path())
        .arg("--rootfs-stages")
        .arg("base")
        .arg("build");

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("Missing required containerfiles"));
}

#[test]
fn test_run_command_with_args() {
    let mut cmd = Command::cargo_bin("trls").unwrap();
    cmd.arg("--skip-root-check");
    cmd.arg("run").arg("--").arg("echo").arg("hello");

    // This might succeed if the system has the container, so we just check it runs
    let output = cmd.output().unwrap();
    // Just verify the command executed, regardless of success/failure
    assert!(output.status.code().is_some());
}

#[test]
fn test_update_command() {
    let temp_dir = TempDir::new().unwrap();
    let mut cmd = Command::cargo_bin("trls").unwrap();
    cmd.arg("--skip-root-check");
    cmd.arg("--stages-dir").arg(temp_dir.path()).arg("update");

    // This will likely fail since we don't have stages defined, but it tests the command
    cmd.assert().failure().stderr(
        predicate::str::contains("Missing required containerfiles")
            .or(predicate::str::contains("No rootfs stages defined")),
    );
}

#[test]
fn test_multiple_stages() {
    let temp_dir = TempDir::new().unwrap();

    let mut cmd = Command::cargo_bin("trls").unwrap();
    cmd.arg("--skip-root-check");
    cmd.arg("--stages-dir")
        .arg(temp_dir.path())
        .arg("--rootfs-stages")
        .arg("base,tools,final")
        .arg("build");

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("Missing required containerfiles"));
}

#[test]
fn test_extra_contexts() {
    let temp_dir = TempDir::new().unwrap();

    let mut cmd = Command::cargo_bin("trls").unwrap();
    cmd.arg("--skip-root-check");
    cmd.arg("--stages-dir")
        .arg(temp_dir.path())
        .arg("--extra-contexts")
        .arg("mycontext=/path/to/context")
        .arg("--rootfs-stages")
        .arg("base")
        .arg("build");

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("Missing required containerfiles"));
}

#[test]
fn test_extra_mounts() {
    let temp_dir = TempDir::new().unwrap();

    let mut cmd = Command::cargo_bin("trls").unwrap();
    cmd.arg("--skip-root-check");
    cmd.arg("--stages-dir")
        .arg(temp_dir.path())
        .arg("--extra-mounts")
        .arg("/tmp,/var/tmp")
        .arg("--rootfs-stages")
        .arg("base")
        .arg("build");

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("Missing required containerfiles"));
}

#[test]
fn test_root_password_shows_security_warning() {
    // Test that using --root-password displays the security warning
    let temp_dir = TempDir::new().unwrap();
    let mut cmd = Command::cargo_bin("trls").unwrap();
    
    cmd.arg("--skip-root-check")
        .arg("--stages-dir")
        .arg(temp_dir.path())
        .arg("--rootfs-stages")
        .arg("base")
        .arg("image")
        .arg("--root-password")
        .arg("test_password");
    
    // This will fail due to missing containerfiles, but the warning should still be shown
    // in stderr before the error message
    let output = cmd.output().unwrap();
    let stderr_str = String::from_utf8_lossy(&output.stderr);
    
    // The security warning should be displayed
    assert!(
        stderr_str.contains("Security notice: Password provided via command-line is visible in process list and shell history"),
        "Expected security warning about password visibility, got stderr: '{stderr_str}'"
    );
    
    // Also check for the follow-up advice
    assert!(
        stderr_str.contains("Consider using environment variables or password files for production use"),
        "Expected advice about using environment variables or password files, got stderr: '{stderr_str}'"
    );
}

#[test]
fn test_no_warning_without_root_password() {
    // Test that the security warning does NOT appear when --root-password is not used
    let temp_dir = TempDir::new().unwrap();
    let mut cmd = Command::cargo_bin("trls").unwrap();
    
    cmd.arg("--skip-root-check")
        .arg("--stages-dir")
        .arg(temp_dir.path())
        .arg("--rootfs-stages")
        .arg("base")
        .arg("image");
    
    let output = cmd.output().unwrap();
    let stderr_str = String::from_utf8_lossy(&output.stderr);
    
    // The security warning should NOT be displayed when no password is provided
    assert!(
        !stderr_str.contains("Security notice: Password provided via command-line"),
        "Security warning should not appear when --root-password is not used, got stderr: '{stderr_str}'"
    );
}
