pub fn target_suffix() -> &'static str {
    match () {
        _ if cfg!(all(target_os = "windows", target_arch = "x86_64")) => "windows-x86_64.exe",
        _ if cfg!(all(target_os = "linux", target_arch = "x86_64")) => "linux-x86_64",
        _ if cfg!(all(target_os = "macos", target_arch = "x86_64")) => "macos-x86_64",
        _ if cfg!(all(target_os = "macos", target_arch = "aarch64")) => "macos-arm64",
        _ => "unknown",
    }
}