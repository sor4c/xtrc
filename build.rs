//! Build script embedding the deterministic version policy into `XTRC_BUILD_VERSION`.
//!
//! // @trace SPEC-VER-POLICY, REQ-VER-POLICY, REQ-VER-BUMP-PATCH, SPEC-VER-BUMP

use std::process::Command;

fn compute_git_version(base_version: &str) -> String {
    if let Ok(explicit) = std::env::var("XTRC_DELIVERY_VERSION") {
        let trimmed = explicit.trim();
        if !trimmed.is_empty() {
            return trimmed.to_string();
        }
    }

    let exact_tag = Command::new("git")
        .args(["describe", "--exact-match", "--tags", "HEAD"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string());

    if exact_tag.as_deref() == Some(&format!("v{base_version}")) {
        return base_version.to_string();
    }

    let commit_count = Command::new("git")
        .args(["rev-list", "--count", "HEAD"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string());

    let short_sha = Command::new("git")
        .args(["rev-parse", "--short=7", "HEAD"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string());

    match (commit_count, short_sha) {
        (Some(count), Some(sha)) if !count.is_empty() && !sha.is_empty() => {
            format!("{base_version}-r{count}.g{sha}")
        }
        _ => base_version.to_string(),
    }
}

fn main() {
    println!("cargo:rerun-if-env-changed=XTRC_DELIVERY_VERSION");
    println!("cargo:rerun-if-changed=.git/HEAD");
    println!("cargo:rerun-if-changed=.git/refs");

    let base_version = std::env::var("CARGO_PKG_VERSION").unwrap_or_else(|_| "0.1.2".to_string());
    let build_version = compute_git_version(&base_version);
    println!("cargo:rustc-env=XTRC_BUILD_VERSION={build_version}");
}
