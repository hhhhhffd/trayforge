//! Prints every `ToolInfo` as JSON, for UI development outside the Tauri shell:
//! `cargo run -p trayforge-core --example dump_tools > ui/public/dev-tools.json`
fn main() {
    let infos: Vec<_> = trayforge_core::registry().iter().map(|t| &t.info).collect();
    println!(
        "{}",
        serde_json::to_string_pretty(&infos).expect("ToolInfo serializes")
    );
}
