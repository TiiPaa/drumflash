//! [280] Starter presets: every JSON file under `assets/presets/default-user/`
//! is deflated into `OUT_DIR` (7 MB of pretty-printed JSON → ~22 KB) and
//! listed in `default_user_presets.rs`, included by `src/presets.rs`. Adding a
//! starter preset = dropping its file in the matching subfolder.

use std::path::{Path, PathBuf};

/// Same folder names as `presets::PresetKind::subdir()`.
const SUBDIRS: [&str; 4] = ["instruments", "patterns", "grids", "songs"];

fn main() {
    let src = Path::new("assets/presets/default-user");
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed={}", src.display());

    let out = PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR is set by cargo"));
    let mut list = String::from("&[\n");
    for subdir in SUBDIRS {
        let Ok(entries) = std::fs::read_dir(src.join(subdir)) else {
            continue;
        };
        let mut names: Vec<String> = entries
            .flatten()
            .filter_map(|e| e.file_name().into_string().ok())
            .filter(|n| n.ends_with(".json"))
            .collect();
        names.sort();
        for name in names {
            let json = std::fs::read(src.join(subdir).join(&name))
                .unwrap_or_else(|e| panic!("read {subdir}/{name}: {e}"));
            let packed = miniz_oxide::deflate::compress_to_vec(&json, 10);
            let packed_path = out.join(format!("{subdir}-{name}.deflate"));
            std::fs::write(&packed_path, packed)
                .unwrap_or_else(|e| panic!("write {}: {e}", packed_path.display()));
            list.push_str(&format!(
                "    ({subdir:?}, {name:?}, include_bytes!({:?})),\n",
                packed_path.display().to_string()
            ));
        }
    }
    list.push_str("]\n");
    std::fs::write(out.join("default_user_presets.rs"), list)
        .expect("write default_user_presets.rs");
}
