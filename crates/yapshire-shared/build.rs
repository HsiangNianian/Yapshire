use std::{env, fs, path::Path};

fn collect(root: &Path, dir: &Path, paths: &mut Vec<String>) {
    for entry in fs::read_dir(dir).expect("Read official content pack") {
        let entry = entry.unwrap();
        let kind = entry.file_type().unwrap();
        assert!(
            !kind.is_symlink(),
            "Bundled content cannot contain symlinks"
        );
        let path = entry.path();
        if kind.is_dir() {
            collect(root, &path, paths);
        } else if kind.is_file() {
            paths.push(
                path.strip_prefix(root)
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .replace('\\', "/"),
            );
        }
    }
}
fn main() {
    let root = Path::new("../../assets/packs/yapshire")
        .canonicalize()
        .unwrap();
    println!("cargo:rerun-if-changed={}", root.display());
    let mut files = Vec::new();
    collect(&root, &root, &mut files);
    files.sort();
    let mut source = String::from("pub const BUNDLED_FILES: &[(&str, &[u8])] = &[\n");
    for path in files {
        source.push_str(&format!(
            "({path:?}, include_bytes!({:?})),\n",
            root.join(&path)
        ));
    }
    source.push_str("];\n");
    fs::write(
        Path::new(&env::var("OUT_DIR").unwrap()).join("bundled.rs"),
        source,
    )
    .unwrap();
}
