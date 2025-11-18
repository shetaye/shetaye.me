use std::fs;
use std::path::Path;
use std::process::Command;

fn main() {
    let tailwind_binary = get_or_download_tailwind();

    // Run Tailwind CSS compilation
    let status = Command::new(&tailwind_binary)
        .args(&[
            "-i", "static/input.css",
            "-o", "static/style.css",
            "--minify",
        ])
        .status()
        .expect("Failed to run Tailwind CSS");

    if !status.success() {
        panic!("Tailwind CSS compilation failed");
    }

    // Tell Cargo to rerun this build script if these files change
    println!("cargo:rerun-if-changed=static/input.css");
    println!("cargo:rerun-if-changed=tailwind.config.js");
    println!("cargo:rerun-if-changed=src/main.rs"); // Since maud templates are here
}

fn get_or_download_tailwind() -> String {
    let binary_name = if cfg!(windows) {
        "tailwindcss.exe"
    } else {
        "tailwindcss"
    };

    let binary_path = format!("./tools/{}", binary_name);

    // If binary exists, use it
    if Path::new(&binary_path).exists() {
        return binary_path;
    }

    // Otherwise, try to use system-installed tailwindcss
    if which::which(binary_name).is_ok() {
        return binary_name.to_string();
    }

    // Download if not present
    println!("cargo:warning=Downloading Tailwind CSS standalone binary...");

    let download_url = get_tailwind_download_url();
    let tools_dir = Path::new("./tools");
    fs::create_dir_all(tools_dir).expect("Failed to create tools directory");

    // Download the binary
    let response = ureq::get(&download_url)
        .call()
        .expect("Failed to download Tailwind CSS");

    let mut file = fs::File::create(&binary_path)
        .expect("Failed to create Tailwind binary file");

    std::io::copy(&mut response.into_reader(), &mut file)
        .expect("Failed to save Tailwind binary");

    // Make executable on Unix
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&binary_path)
            .expect("Failed to get file metadata")
            .permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&binary_path, perms)
            .expect("Failed to set executable permissions");
    }

    binary_path
}

fn get_tailwind_download_url() -> String {
    let version = "v3.4.17"; // Update as needed

    let platform = if cfg!(target_os = "linux") {
        if cfg!(target_arch = "x86_64") {
            "linux-x64"
        } else if cfg!(target_arch = "aarch64") {
            "linux-arm64"
        } else {
            panic!("Unsupported Linux architecture")
        }
    } else if cfg!(target_os = "macos") {
        if cfg!(target_arch = "x86_64") {
            "macos-x64"
        } else if cfg!(target_arch = "aarch64") {
            "macos-arm64"
        } else {
            panic!("Unsupported macOS architecture")
        }
    } else if cfg!(target_os = "windows") {
        if cfg!(target_arch = "x86_64") {
            "windows-x64.exe"
        } else {
            panic!("Unsupported Windows architecture")
        }
    } else {
        panic!("Unsupported operating system")
    };

    format!(
        "https://github.com/tailwindlabs/tailwindcss/releases/download/{}/tailwindcss-{}",
        version, platform
    )
}
