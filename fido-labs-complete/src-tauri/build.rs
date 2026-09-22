// FIDO-LABS build script (Tauri 2.x)
fn main() {
    tauri_build::build();

    #[cfg(target_os = "windows")]
    {
        // Configuration Windows spécifique
        println!("cargo:warning=FIDO-LABS: Build Windows");
    }
    #[cfg(target_os = "linux")]
    {
        println!("cargo:warning=FIDO-LABS: Build Linux");
    }
    #[cfg(target_os = "macos")]
    {
        println!("cargo:warning=FIDO-LABS: Build macOS");
    }
}
