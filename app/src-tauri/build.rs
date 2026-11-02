fn main() {
  let mut attributes = tauri_build::Attributes::new();
  // release: always run as administrator (nous-admin.manifest); debug keeps
  // the default so `tauri dev` starts from a normal terminal
  if std::env::var("PROFILE").as_deref() == Ok("release") {
    println!("cargo:rerun-if-changed=nous-admin.manifest");
    attributes = attributes
      .windows_attributes(tauri_build::WindowsAttributes::new().app_manifest(include_str!("nous-admin.manifest")));
  }
  tauri_build::try_build(attributes).expect("tauri build");
}
