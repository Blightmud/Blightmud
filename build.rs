use std::process::Command;
fn main() {
    // taken from https://stackoverflow.com/questions/43753491/include-git-commit-hash-as-string-into-rust-program
    let git_hash = if let Ok(output) = Command::new("git")
        .args(&["rev-parse", "--short", "HEAD"])
        .output()
    {
        String::from_utf8(output.stdout).unwrap_or_default()
    } else {
        String::new()
    };
    println!("cargo:rustc-env=GIT_HASH={}", git_hash);
}
