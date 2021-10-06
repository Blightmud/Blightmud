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

    let git_tag = if let Ok(output) = Command::new("git")
        .args(&["describe", "--exact-match", "--tags", "HEAD"])
        .output()
    {
        String::from_utf8(output.stdout).unwrap_or_default()
    } else {
        String::new()
    };
    println!("cargo:rustc-env=GIT_TAG={}", git_tag);

    if git_tag.is_empty() {
        let git_describe =
            if let Ok(output) = Command::new("git").args(&["describe", "--tags"]).output() {
                String::from_utf8(output.stdout).unwrap_or_default()
            } else {
                String::new()
            };

        let git_describe_format = if git_describe.is_empty() {
            String::new()
        } else {
            format!(" ({})", git_describe.trim())
        };

        println!("cargo:rustc-env=GIT_DESCRIBE={}", git_describe_format);
    } else {
        println!("cargo:rustc-env=GIT_DESCRIBE=");
    }
}
