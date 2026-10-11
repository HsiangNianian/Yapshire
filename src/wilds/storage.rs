use std::{
    io,
    path::{Path, PathBuf},
};
use yapshire_shared::{content::read_limited, terrain::Terrain};

const MAX_SAVE_BYTES: u64 = 32 * 1024 * 1024;

pub(crate) fn directory() -> io::Result<PathBuf> {
    std::env::var_os("YAPSHIRE_WORLD_DIR")
        .map(PathBuf::from)
        .map_or_else(|| crate::paths::data_dir().map(|p| p.join("worlds")), Ok)
}

pub(crate) struct Entry {
    pub path: PathBuf,
    pub name: String,
    pub seed: Option<u32>,
}

pub(crate) fn list(directory: &Path) -> io::Result<Vec<Entry>> {
    let files = match std::fs::read_dir(directory) {
        Ok(files) => files,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(e),
    };
    let mut entries = Vec::new();
    for file in files {
        let file = file?;
        if !file.file_type()?.is_file() || file.path().extension().is_none_or(|s| s != "json") {
            continue;
        }
        let world = Saved::load(&file.path()).ok();
        entries.push(Entry {
            path: file.path(),
            name: world.as_ref().map_or_else(
                || file.file_name().to_string_lossy().into_owned(),
                |w| w.terrain.name.clone(),
            ),
            seed: world.map(|w| w.terrain.seed),
        });
    }
    entries.sort_by(|a, b| a.name.cmp(&b.name).then(a.path.cmp(&b.path)));
    Ok(entries)
}

pub(crate) struct Saved {
    pub terrain: Terrain,
    path: PathBuf,
    baseline: Vec<u8>,
}

impl Saved {
    pub fn create(directory: &Path, terrain: Terrain) -> io::Result<Self> {
        use std::io::Write;
        terrain.validate()?;
        std::fs::create_dir_all(directory)?;
        let path = directory.join(format!("{:016x}.json", rand::random::<u64>()));
        let baseline = serde_json::to_vec(&terrain)?;
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)?;
        file.write_all(&baseline)?;
        file.sync_all()?;
        Ok(Self {
            terrain,
            path,
            baseline,
        })
    }

    pub fn load(path: &Path) -> io::Result<Self> {
        let baseline = read_limited(path, MAX_SAVE_BYTES)?;
        let terrain: Terrain = serde_json::from_slice(&baseline)?;
        terrain.validate()?;
        Ok(Self {
            terrain,
            path: path.to_owned(),
            baseline,
        })
    }

    pub fn save(&mut self) -> io::Result<()> {
        self.terrain.validate()?;
        // Never silently overwrite a save changed by another running game/editor.
        if read_limited(&self.path, MAX_SAVE_BYTES)? != self.baseline {
            return Err(io::Error::other(
                "World file changed outside this game; your in-memory world is still open",
            ));
        }
        let bytes = serde_json::to_vec(&self.terrain)?;
        if bytes.len() as u64 > MAX_SAVE_BYTES {
            return Err(io::Error::other("World exceeds the 32 MiB save limit"));
        }
        if bytes != self.baseline {
            crate::paths::atomic_write(&self.path, &bytes)?;
            self.baseline = bytes;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn saves_are_isolated_and_corruption_or_external_changes_are_not_overwritten() {
        let dir = std::env::temp_dir().join(format!("yapshire-world-{}", rand::random::<u64>()));
        let mut a = Saved::create(&dir, Terrain::new("../World".into(), 42)).unwrap();
        let b = Saved::create(&dir, Terrain::new("../World".into(), 42)).unwrap();
        assert_ne!(a.path, b.path);
        assert_eq!(a.path.parent(), Some(dir.as_path()));
        let p = a.terrain.position;
        assert!(a.terrain.dig(1026, 160, p));
        a.save().unwrap();
        assert_eq!(Saved::load(&a.path).unwrap().terrain, a.terrain);
        assert!(Saved::load(&b.path).unwrap().terrain.changes.is_empty());
        std::fs::write(&a.path, b"broken").unwrap();
        assert!(a.save().is_err());
        assert!(Saved::load(&a.path).is_err());
        assert_eq!(std::fs::read(&a.path).unwrap(), b"broken");
        assert_eq!(list(&dir).unwrap().len(), 2);
        std::fs::remove_dir_all(dir).unwrap();
    }
}
