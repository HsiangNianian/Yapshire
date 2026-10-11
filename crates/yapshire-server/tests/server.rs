use std::{
    io::{Read, Write},
    net::{SocketAddr, TcpListener, TcpStream},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};
use tungstenite::{Message, WebSocket, client::IntoClientRequest};
use yapshire_server::{Config, Server};
use yapshire_shared::{
    World,
    protocol::{ClientMessage, ServerMessage, authorization},
};

struct Running {
    address: SocketAddr,
    stop: Arc<AtomicBool>,
    thread: Option<std::thread::JoinHandle<()>>,
    server: Server,
}
impl Running {
    fn new(config: Config, world: World, password: &str) -> Self {
        let server = Server::new(config, world, password).unwrap();
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let stop = Arc::new(AtomicBool::new(false));
        let thread = Some(server.clone().spawn(listener, stop.clone()).unwrap());
        Self {
            address,
            stop,
            thread,
            server,
        }
    }
    fn request(
        &self,
        path: &str,
        password: &str,
        origin: Option<&str>,
    ) -> tungstenite::handshake::client::Request {
        let mut request = format!("ws://{}{path}", self.address)
            .into_client_request()
            .unwrap();
        if !password.is_empty() {
            request
                .headers_mut()
                .insert("Authorization", authorization(password).parse().unwrap());
        }
        if let Some(origin) = origin {
            request
                .headers_mut()
                .insert("Origin", origin.parse().unwrap());
        }
        request
    }
    fn open(&self, path: &str, password: &str) -> WebSocket<TcpStream> {
        let stream = TcpStream::connect(self.address).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(3)))
            .unwrap();
        stream
            .set_write_timeout(Some(Duration::from_secs(3)))
            .unwrap();
        tungstenite::client(self.request(path, password, None), stream)
            .unwrap()
            .0
    }
    fn rejected(&self, path: &str, password: &str, origin: Option<&str>, status: u16) {
        let stream = TcpStream::connect(self.address).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(3)))
            .unwrap();
        let result = tungstenite::client(self.request(path, password, origin), stream);
        match result {
            Err(tungstenite::HandshakeError::Failure(tungstenite::Error::Http(response))) => {
                assert_eq!(response.status().as_u16(), status)
            }
            other => panic!("Expected HTTP {status}, got {other:?}"),
        }
    }
    fn http(&self, path: &str, password: &str) -> String {
        let mut stream = TcpStream::connect(self.address).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(3)))
            .unwrap();
        write!(stream, "GET {path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\nAuthorization: {}\r\n\r\n", authorization(password)).unwrap();
        let mut response = String::new();
        stream.read_to_string(&mut response).unwrap();
        response
    }
}
impl Drop for Running {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        self.thread.take().unwrap().join().unwrap();
    }
}
fn read(socket: &mut WebSocket<TcpStream>) -> ServerMessage {
    serde_json::from_str(socket.read().unwrap().to_text().unwrap()).unwrap()
}
fn acknowledge(socket: &mut WebSocket<TcpStream>) -> World {
    let ServerMessage::World { world } = read(socket) else {
        panic!("Maps must arrive before admission")
    };
    world.validate().unwrap();
    socket
        .send(Message::text(
            serde_json::to_string(&ClientMessage::WorldReady {
                revision: world.revision.clone(),
            })
            .unwrap(),
        ))
        .unwrap();
    *world
}
fn joined(socket: &mut WebSocket<TcpStream>) -> u32 {
    let ServerMessage::Welcome { you, .. } = read(socket) else {
        panic!("Expected welcome")
    };
    you
}

#[test]
fn lobby_probe_is_authenticated_reports_capacity_and_never_joins_a_room() {
    let host = Running::new(
        Config {
            max_players: 3,
            ..Config::default()
        },
        World::bundled(),
        "club-password",
    );
    host.rejected("/lobby?probe=1", "", None, 401);
    host.rejected(
        "/lobby?probe=1",
        "club-password",
        Some("https://untrusted.example"),
        403,
    );
    let mut socket = host.open("/lobby?probe=1", "club-password");
    let listing: serde_json::Value =
        serde_json::from_str(socket.read().unwrap().to_text().unwrap()).unwrap();
    assert_eq!(listing["probe"], true);
    assert_eq!(listing["rooms"][0]["capacity"], 3);
    assert_eq!(listing["rooms"][0]["players"], 0);
    socket.send(Message::text("ping")).unwrap();
    assert_eq!(socket.read().unwrap().to_text().unwrap(), "pong");
    assert!(matches!(socket.read().unwrap(), Message::Close(_)));
    assert_eq!(host.server.player_count("MAIN0001"), 0);
    let mut old_client = host.open("/lobby", "club-password");
    let listing: serde_json::Value =
        serde_json::from_str(old_client.read().unwrap().to_text().unwrap()).unwrap();
    assert_eq!(listing["probe"], false);
    assert!(matches!(old_client.read().unwrap(), Message::Close(_)));
}

#[test]
fn two_clients_share_edited_tiled_maps_and_server_assigned_identities() {
    let mut world = World::bundled();
    world.maps.get_mut("yapshire:town").unwrap().layers[4].data[500] = 0x8000_005d;
    world.maps.get_mut("yapshire:tackle_shop").unwrap().layers[4].data[100] = 12;
    let world = World::from_maps(world.content, world.maps).unwrap();
    let host = Running::new(Config::default(), world.clone(), "");
    let mut a = host.open("/room/MAIN0001?protocol=3&name=Alice", "");
    assert_eq!(acknowledge(&mut a), world);
    let alice = joined(&mut a);
    let mut b = host.open("/room/MAIN0001?protocol=3&name=Bob", "");
    assert_eq!(acknowledge(&mut b), world);
    let bob = joined(&mut b);
    assert_ne!(alice, bob);
    assert!(matches!(read(&mut a), ServerMessage::Joined { player } if player.id == bob));
    a.send(Message::text(format!(
        r#"{{"type":"chat","id":{bob},"text":"你好，自建小镇！"}}"#
    )))
    .unwrap();
    assert!(
        matches!(read(&mut b), ServerMessage::Chat { id, text } if id == alice && text == "你好，自建小镇！")
    );
    assert!(matches!(read(&mut a), ServerMessage::Chat { id, .. } if id == alice));
    a.send(Message::text(r#"{"type":"move","map":"yapshire:tackle_shop","x":99999,"y":-99,"moving":true,"facing":false,"indoors":true,"fishing":true}"#)).unwrap();
    assert!(
        matches!(read(&mut b), ServerMessage::Moved { player } if player.id == alice && player.map == "yapshire:tackle_shop" && player.x == 468.0 && player.y == -64.0 && player.indoors && !player.fishing)
    );
    a.close(None).unwrap();
    drop(a);
    assert!(matches!(read(&mut b), ServerMessage::Left { id } if id == alice));
    let lobby = host.http("/rooms", "");
    assert!(
        lobby.starts_with("HTTP/1.1 200")
            && lobby.contains("MAIN0001")
            && lobby.contains("\"players\":1")
    );
    b.close(None).unwrap();
    drop(b);
    for _ in 0..30 {
        if host.server.player_count("MAIN0001") == 0 {
            break;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(
        host.http("/rooms", "").contains("MAIN0001"),
        "The configured town survives empty rooms"
    );
}

#[test]
fn passwords_origins_protocol_and_per_address_capacity_are_enforced() {
    let host = Running::new(
        Config {
            max_connections_per_ip: 1,
            ..Config::default()
        },
        World::bundled(),
        "test-password",
    );
    let path = "/room/MAIN0001?protocol=3&name=Alice";
    host.rejected(path, "", None, 401);
    host.rejected(path, "wrong-password", None, 401);
    host.rejected(path, "test-password", Some("https://foreign.example"), 403);
    host.rejected("/room/MAIN0001?protocol=1", "test-password", None, 426);
    host.rejected("/room/MAIN0001?protocol=2", "test-password", None, 426);
    assert!(host.http("/rooms", "").starts_with("HTTP/1.1 401"));
    assert!(host.http("/health", "").contains("\"protocol\":3"));
    let mut socket = host.open(path, "test-password");
    acknowledge(&mut socket);
    joined(&mut socket);
    host.rejected(path, "test-password", None, 429);
    assert!(!host.http("/health", "").contains("test-password"));
}

#[test]
fn incorrect_map_acknowledgement_never_creates_a_player() {
    let host = Running::new(Config::default(), World::bundled(), "");
    let mut socket = host.open("/room/MAIN0001?protocol=3", "");
    assert!(matches!(read(&mut socket), ServerMessage::World { .. }));
    socket
        .send(Message::text(
            r#"{"type":"world_ready","revision":"wrong"}"#,
        ))
        .unwrap();
    assert!(
        matches!(socket.read().unwrap(), Message::Close(Some(frame)) if u16::from(frame.code) == 1008)
    );
    assert_eq!(host.server.player_count("MAIN0001"), 0);
}

#[test]
fn create_races_capacity_and_room_isolation_preserve_existing_players() {
    let host = Running::new(
        Config {
            max_rooms: 2,
            max_players: 1,
            ..Config::default()
        },
        World::bundled(),
        "",
    );
    let path = "/room/EXTRA001?protocol=3&create=1&room_name=Friends&name=Alice";
    let mut a = host.open(path, "");
    let mut race = host.open(path, "");
    acknowledge(&mut a);
    let id = joined(&mut a);
    acknowledge(&mut race);
    assert!(
        matches!(race.read().unwrap(), Message::Close(Some(frame)) if u16::from(frame.code) == 1008)
    );
    host.rejected("/room/EXTRA001?protocol=3", "", None, 409);
    host.rejected(
        "/room/EXTRA002?protocol=3&create=1&room_name=Other",
        "",
        None,
        503,
    );
    let mut b = host.open("/room/MAIN0001?protocol=3", "");
    acknowledge(&mut b);
    joined(&mut b);
    a.send(Message::text(r#"{"type":"chat","text":"Only my room"}"#))
        .unwrap();
    assert!(matches!(read(&mut a), ServerMessage::Chat { id: speaker, .. } if speaker == id));
    b.get_mut()
        .set_read_timeout(Some(Duration::from_millis(100)))
        .unwrap();
    assert!(
        matches!(b.read(), Err(tungstenite::Error::Io(error)) if matches!(error.kind(), std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut))
    );
    a.close(None).unwrap();
    drop(a);
    for _ in 0..30 {
        if host.server.player_count("EXTRA001") == 0 {
            break;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    host.rejected("/room/EXTRA001?protocol=3", "", None, 404);
}

#[test]
fn malformed_messages_and_shutdown_close_connections() {
    let host = Running::new(Config::default(), World::bundled(), "");
    let mut a = host.open("/room/MAIN0001?protocol=3", "");
    acknowledge(&mut a);
    joined(&mut a);
    a.send(Message::text(
        r#"{"type":"move","map":"yapshire:town","x":0,"y":0,"moving":"yes","facing":false}"#,
    ))
    .unwrap();
    assert!(
        matches!(a.read().unwrap(), Message::Close(Some(frame)) if u16::from(frame.code) == 1007)
    );
    let mut b = host.open("/room/MAIN0001?protocol=3", "");
    acknowledge(&mut b);
    joined(&mut b);
    let mut pending = host.open("/room/MAIN0001?protocol=3", "");
    assert!(matches!(read(&mut pending), ServerMessage::World { .. }));
    host.stop.store(true, Ordering::Relaxed);
    assert!(matches!(b.read().unwrap(), Message::Close(_)));
    assert!(matches!(pending.read().unwrap(), Message::Close(_)));
}

#[test]
fn oversized_binary_and_flooding_clients_do_not_interrupt_the_room() {
    let host = Running::new(Config::default(), World::bundled(), "");
    let mut observer = host.open("/room/MAIN0001?protocol=3&name=Observer", "");
    acknowledge(&mut observer);
    let observer_id = joined(&mut observer);
    for (message, code) in [
        (Message::text("x".repeat(2049)), 1009),
        (Message::Binary(vec![1, 2].into()), 1003),
    ] {
        let mut offender = host.open("/room/MAIN0001?protocol=3", "");
        acknowledge(&mut offender);
        let offender_id = joined(&mut offender);
        assert!(matches!(read(&mut observer), ServerMessage::Joined { .. }));
        offender.send(message).unwrap();
        assert!(
            matches!(offender.read().unwrap(), Message::Close(Some(frame)) if u16::from(frame.code) == code)
        );
        assert!(matches!(read(&mut observer), ServerMessage::Left { id } if id == offender_id));
    }
    let mut flood = host.open("/room/MAIN0001?protocol=3", "");
    acknowledge(&mut flood);
    let flood_id = joined(&mut flood);
    assert!(matches!(read(&mut observer), ServerMessage::Joined { .. }));
    for _ in 0..90 {
        if flood.send(Message::text("ping")).is_err() {
            break;
        }
    }
    assert!(matches!(read(&mut observer), ServerMessage::Left { id } if id == flood_id));
    observer
        .send(Message::text(r#"{"type":"chat","text":"Still connected"}"#))
        .unwrap();
    assert!(matches!(read(&mut observer), ServerMessage::Chat { id, .. } if id == observer_id));
    assert_eq!(host.server.player_count("MAIN0001"), 1);
}

#[test]
fn standalone_cli_initializes_and_checks_the_same_editor_map_files() {
    let folder =
        std::env::temp_dir().join(format!("yapshire-server-test-{}", rand::random::<u64>()));
    let binary = env!("CARGO_BIN_EXE_yapshire-server");
    let initialize = std::process::Command::new(binary)
        .arg("--init")
        .arg(&folder)
        .output()
        .unwrap();
    assert!(initialize.status.success(), "{:?}", initialize);
    let config_path = folder.join("server.json");
    let config = Config::load(&config_path).unwrap();
    assert_eq!(config.world().unwrap(), World::bundled());
    // Windows users may already have CRLF copies from previous downloads.
    let metadata = folder.join("maps/terrain/ground.tsj");
    let crlf = std::fs::read_to_string(&metadata)
        .unwrap()
        .replace("\r\n", "\n")
        .replace('\n', "\r\n");
    std::fs::write(&metadata, &crlf).unwrap();
    assert_eq!(config.world().unwrap(), World::bundled());
    std::fs::write(&metadata, b"{}\n").unwrap();
    assert!(config.world().is_err());
    std::fs::write(&metadata, crlf).unwrap();
    let town_file = folder.join("maps/maps/town.tmj");
    let mut town = config.world().unwrap().maps["yapshire:town"].clone();
    town.layers[4].data[500] = 0x8000_005d;
    let edited = serde_json::to_vec(&town).unwrap();
    std::fs::write(&town_file, &edited).unwrap();
    let check = std::process::Command::new(binary)
        .current_dir(std::env::temp_dir())
        .arg("--config")
        .arg(&config_path)
        .arg("--check")
        .output()
        .unwrap();
    assert!(check.status.success(), "{:?}", check);
    assert!(
        String::from_utf8(check.stdout)
            .unwrap()
            .contains(&config.world().unwrap().revision)
    );
    let reinitialize = std::process::Command::new(binary)
        .arg("--init")
        .arg(&folder)
        .output()
        .unwrap();
    assert!(!reinitialize.status.success());
    assert_eq!(std::fs::read(&town_file).unwrap(), edited);
    let mut filled = town.clone();
    for layer in &mut filled.layers {
        layer.data.fill(0x8000_005d);
    }
    let pretty = serde_json::to_vec_pretty(&filled).unwrap();
    assert!(
        pretty.len() > 128 * 1024,
        "Exercise a fully painted, flipped editor export"
    );
    std::fs::write(&town_file, pretty).unwrap();
    assert!(config.world().is_ok());
    let palette = folder.join("maps/objects/harbor.png");
    std::fs::write(&palette, b"not the shared palette").unwrap();
    assert!(config.world().is_err());
    std::fs::write(
        &palette,
        yapshire_shared::bundled_file("objects/harbor.png").unwrap(),
    )
    .unwrap();
    std::fs::write(
        &town_file,
        vec![b' '; yapshire_shared::MAX_MAP_BYTES as usize + 1],
    )
    .unwrap();
    assert!(config.world().is_err());
    town.layers[0].data[0] = 999;
    std::fs::write(&town_file, serde_json::to_vec(&town).unwrap()).unwrap();
    let check = std::process::Command::new(binary)
        .arg("--config")
        .arg(&config_path)
        .arg("--check")
        .output()
        .unwrap();
    assert!(!check.status.success());
    std::fs::remove_dir_all(folder).unwrap();
}
