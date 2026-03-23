use std::path::Path;
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=src/db/migrations");

    // Early exit if browser feature is not enabled — no npm needed
    if std::env::var("CARGO_FEATURE_BROWSER").is_err() {
        return;
    }

    println!("cargo::rerun-if-changed=web/src");
    println!("cargo::rerun-if-changed=web/package.json");

    // Check npm availability
    let npm_check = Command::new("npm").arg("--version").output();
    match npm_check {
        Ok(output) if output.status.success() => {}
        _ => {
            println!("cargo::error=npm not found. Install Node.js from https://nodejs.org to build the browser UI.");
            std::process::exit(1);
        }
    }

    // Run npm install in web/
    let install_status = Command::new("npm")
        .arg("install")
        .current_dir(Path::new("web"))
        .status();
    match install_status {
        Ok(status) if status.success() => {}
        _ => {
            println!("cargo::error=npm install failed in web/");
            std::process::exit(1);
        }
    }

    // Run npm run build in web/
    let build_status = Command::new("npm")
        .arg("run")
        .arg("build")
        .current_dir(Path::new("web"))
        .status();
    match build_status {
        Ok(status) if status.success() => {}
        _ => {
            println!(
                "cargo::error=npm run build failed in web/. Check web/src for TypeScript errors."
            );
            std::process::exit(1);
        }
    }
}
