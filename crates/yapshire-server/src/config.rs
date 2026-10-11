use serde::{Deserialize, Serialize};
use std::{
    io,
    net::SocketAddr,
    path::{Path, PathBuf},
};
use yapshire_shared::{World, protocol::clean};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub bind: SocketAddr,
    pub name: String,
    pub room_code: String,
    pub maps_dir: Option<PathBuf>,
    pub max_players: usize,
    pub max_rooms: usize,
    pub max_connections_per_ip: usize,
    pub allow_room_creation: bool,
    pub allowed_origins: Vec<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            bind: "0.0.0.0:4761".parse().unwrap(),
            name: "My Yapshire town".into(),
            room_code: "MAIN0001".into(),
            maps_dir: None,
            max_players: 16,
            max_rooms: 40,
            max_connections_per_ip: 32,
            allow_room_creation: true,
            allowed_origins: Vec::new(),
        }
    }
}

impl Config {
    pub fn load(path: &Path) -> io::Result<Self> {
        let mut config: Self = serde_json::from_slice(&std::fs::read(path)?)?;
        if let Some(folder) = &mut config.maps_dir {
            if folder.is_relative() {
                *folder = path.parent().unwrap_or(Path::new(".")).join(&folder);
            }
        }
        config.validate()?;
        Ok(config)
    }

    pub fn validate(&self) -> io::Result<()> {
        if clean(&self.name, 24).is_empty()
            || self.name.chars().count() > 24
            || !(valid_code(&self.room_code) || self.room_code == "LOCAL")
            || !(1..=16).contains(&self.max_players)
            || !(1..=40).contains(&self.max_rooms)
            || !(1..=256).contains(&self.max_connections_per_ip)
            || self.allowed_origins.len() > 32
            || self.allowed_origins.iter().any(|s| s.len() > 256)
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Invalid server name, room code or limits",
            ));
        }
        Ok(())
    }

    pub fn world(&self) -> io::Result<World> {
        self.maps_dir
            .as_ref()
            .map_or_else(|| Ok(World::bundled()), |folder| World::load(folder))
    }

    pub fn lan(owner: String) -> Self {
        Self {
            name: clean(&owner, 24),
            room_code: "LOCAL".into(),
            max_rooms: 1,
            allow_room_creation: false,
            ..Self::default()
        }
    }

    pub fn initialize(folder: &Path) -> io::Result<()> {
        use std::io::Write;
        let mut files: Vec<(String, Vec<u8>)> = yapshire_shared::BUNDLED_FILES
            .iter()
            .map(|(name, bytes)| (format!("maps/{name}"), bytes.to_vec()))
            .collect();
        files.push((
            "server.json".into(),
            serde_json::to_vec_pretty(&Self {
                maps_dir: Some("maps".into()),
                ..Self::default()
            })?,
        ));
        if files.iter().any(|(name, _)| folder.join(name).exists()) {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                "Initialization never overwrites an existing config or map",
            ));
        }
        std::fs::create_dir_all(folder.join("maps"))?;
        for (name, bytes) in files {
            std::fs::create_dir_all(folder.join(&name).parent().unwrap())?;
            std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(folder.join(name))?
                .write_all(&bytes)?;
        }
        Ok(())
    }
}

pub fn valid_code(code: &str) -> bool {
    code.len() == 8
        && code
            .bytes()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit())
}
