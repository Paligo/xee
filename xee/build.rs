use std::process::Command;

fn main() {
    let output = Command::new("git")
        .args(["log", "-1", "--format=(%cd %h)", "--date=short"])
        .output();

    match output {
        Ok(output) => {
            if let Ok(git_commit) = String::from_utf8(output.stdout) {
                println!("cargo:rustc-env=GIT_COMMIT={}", git_commit.trim());
            }
        }
        Err(e) => {
            eprintln!("Error getting git commit: {}", e);
            println!("cargo:rustc-env=GIT_COMMIT=(- -)");
        }
    }
}
