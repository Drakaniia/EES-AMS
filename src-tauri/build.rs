use std::path::PathBuf;

const MANIFEST_FILE: &str = "comctl32.manifest";

fn main() {
    // Exactly one RT_MANIFEST resource may exist per binary, and only one producer is
    // allowed to create it.
    //
    // `tauri_build::build()` would normally have `tauri-winres` put the manifest into the
    // generated resource file, but `tauri-winres` hands the compiled resource `.lib` to
    // cargo through `cargo:rustc-link-arg-bins`, which applies to `[[bin]]` targets only.
    // So under `cargo test` the `app` lib's unittest binary is linked with no manifest at
    // all, the loader binds its static `comctl32.dll!TaskDialogIndirect` import against
    // `System32\comctl32.dll` (Common Controls v5, which does not export it), and the
    // process dies before `main()` with `STATUS_ENTRYPOINT_NOT_FOUND` (0xc0000139). Cargo
    // then reports a bare "test failed" and zero tests ever execute.
    //
    // Therefore this build script owns manifest embedding for every `link.exe`-linked
    // target (bin, cdylib, examples, benches and tests) and asks tauri-build to leave the
    // manifest out of its resource file, so there is no duplicate to collide with.
    // The manifest content is Tauri 2's own default, `tauri_build`'s
    // `windows-app-manifest.xml`: a Common Controls v6 dependency, which is what makes the
    // Tauri dialog plugin's `TaskDialogIndirect` import resolvable.
    let attributes = tauri_build::Attributes::new()
        .windows_attributes(tauri_build::WindowsAttributes::new_without_app_manifest());
    tauri_build::try_build(attributes).expect("failed to run tauri-build");

    embed_common_controls_manifest();
}

fn embed_common_controls_manifest() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }

    println!("cargo:rerun-if-changed={MANIFEST_FILE}");

    // The compiler's working directory is the crate root rather than this build script's
    // crate root, so `link.exe` has to be handed an absolute path.
    let manifest_dir =
        PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR unset"));
    let manifest = manifest_dir.join(MANIFEST_FILE);

    println!("cargo:rustc-link-arg=/MANIFESTINPUT:{}", manifest.display());
    println!("cargo:rustc-link-arg=/MANIFEST:EMBED");
}
