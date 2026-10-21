// Direct runtime dependencies of Trayforge and their licenses.
// Regenerate by hand when dependencies change: Rust from `cargo metadata --format-version 1`
// (normal deps of src-tauri and crates/trayforge-core), Frontend from ui/node_modules/*/package.json.
export interface LicenseEntry {
  name: string;
  version: string;
  license: string;
}

export const LICENSE_GROUPS: { title: string; entries: LicenseEntry[] }[] = [
  {
    title: "Rust",
    entries: [
      { name: "base64", version: "0.22.1", license: "MIT OR Apache-2.0" },
      { name: "chrono", version: "0.4.45", license: "MIT OR Apache-2.0" },
      { name: "chrono-tz", version: "0.10.4", license: "MIT OR Apache-2.0" },
      { name: "crc", version: "3.4.0", license: "MIT OR Apache-2.0" },
      { name: "csv", version: "1.4.0", license: "Unlicense/MIT" },
      { name: "deunicode", version: "1.6.2", license: "BSD-3-Clause" },
      { name: "fancy-regex", version: "0.19.2", license: "MIT" },
      { name: "figlet-rs", version: "0.1.5", license: "Apache-2.0" },
      { name: "hex", version: "0.4.3", license: "MIT OR Apache-2.0" },
      { name: "hmac", version: "0.12.1", license: "MIT OR Apache-2.0" },
      { name: "image", version: "0.25.10", license: "MIT OR Apache-2.0" },
      { name: "md-5", version: "0.10.6", license: "MIT OR Apache-2.0" },
      { name: "pbkdf2", version: "0.12.2", license: "MIT OR Apache-2.0" },
      { name: "png", version: "0.18.1", license: "MIT OR Apache-2.0" },
      { name: "pulldown-cmark", version: "0.13.4", license: "MIT" },
      { name: "qrcode", version: "0.14.1", license: "MIT OR Apache-2.0" },
      { name: "quick-xml", version: "0.37.5", license: "MIT" },
      { name: "rand", version: "0.9.5", license: "MIT OR Apache-2.0" },
      { name: "reqwest", version: "0.12.28", license: "MIT OR Apache-2.0" },
      { name: "rqrr", version: "0.10.1", license: "(MIT OR Apache-2.0) AND ISC" },
      { name: "serde", version: "1.0.229", license: "MIT OR Apache-2.0" },
      { name: "serde_json", version: "1.0.154", license: "MIT OR Apache-2.0" },
      { name: "serde_yaml", version: "0.9.34", license: "MIT OR Apache-2.0" },
      { name: "sha1", version: "0.10.7", license: "MIT OR Apache-2.0" },
      { name: "sha2", version: "0.10.9", license: "MIT OR Apache-2.0" },
      { name: "tauri", version: "2.12.4", license: "Apache-2.0 OR MIT" },
      { name: "tauri-plugin-autostart", version: "2.7.0", license: "Apache-2.0 OR MIT" },
      { name: "tauri-plugin-dialog", version: "2.8.3", license: "Apache-2.0 OR MIT" },
      { name: "tauri-plugin-global-shortcut", version: "2.4.1", license: "Apache-2.0 OR MIT" },
      { name: "tauri-plugin-single-instance", version: "2.5.4", license: "Apache-2.0 OR MIT" },
      { name: "toml", version: "0.8.2", license: "MIT OR Apache-2.0" },
      { name: "uuid", version: "1.28.0", license: "Apache-2.0 OR MIT" },
      { name: "windows", version: "0.61.3", license: "MIT OR Apache-2.0" },
      { name: "x509-parser", version: "0.18.1", license: "MIT OR Apache-2.0" },
    ],
  },
  {
    title: "Frontend",
    entries: [
      { name: "@tauri-apps/api", version: "2.12.4", license: "Apache-2.0 OR MIT" },
      { name: "@tauri-apps/plugin-dialog", version: "2.8.3", license: "MIT OR Apache-2.0" },
      { name: "lucide-react", version: "1.60.0", license: "ISC" },
      { name: "react", version: "18.3.1", license: "MIT" },
      { name: "react-dom", version: "18.3.1", license: "MIT" },
    ],
  },
];
