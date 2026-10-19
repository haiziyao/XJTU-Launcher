fn main() {
    for key in [
        "XJTU_PLUGIN_SERVER",
        "XJTU_UPDATE_ENDPOINT",
        "XJTU_UPDATE_PUBLIC_KEY",
    ] {
        println!("cargo:rerun-if-env-changed={key}");
    }
    tauri_build::build()
}
