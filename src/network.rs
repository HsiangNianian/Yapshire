use rand::{Rng, distr::Alphanumeric};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    io::{self, Read, Write},
    net::{SocketAddr, TcpListener, TcpStream, ToSocketAddrs, UdpSocket},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver, SyncSender},
    },
    thread,
    time::{Duration, Instant},
};
use tungstenite::{Message, WebSocket, protocol::WebSocketConfig, stream::MaybeTlsStream};
use url::Url;

pub const PORT: u16 = 4761;
pub const DEFAULT_SERVER: &str = include_str!("../assets/server-url.txt");
pub const LEGACY_SERVERS: &[&str] = &[
    "wss://yapshire-multiplayer.opensource-941.workers.dev",
    "wss://niannian-club.opensource-941.workers.dev",
];
const DISCOVERY_PORT: u16 = 4762;
const DISCOVER: &[u8] = b"YAPSHIRE_DISCOVER_V1";

pub use yapshire_shared::protocol::{ClientMessage, Player, RoomEntry, ServerMessage, clean};
use yapshire_shared::{MAX_WORLD_BYTES, PROTOCOL_VERSION, World};

#[derive(Serialize, Deserialize)]
struct LanAnnouncement {
    id: u64,
    name: String,
    players: usize,
    port: u16,
}

pub fn discover_lan() -> Result<Vec<RoomEntry>, String> {
    scan_lan(DISCOVERY_PORT)
}

fn scan_lan(port: u16) -> Result<Vec<RoomEntry>, String> {
    let socket = UdpSocket::bind("0.0.0.0:0").map_err(|e| e.to_string())?;
    socket.set_broadcast(true).map_err(|e| e.to_string())?;
    socket
        .set_read_timeout(Some(Duration::from_millis(200)))
        .map_err(|e| e.to_string())?;
    // A missing broadcast route must not prevent discovering a host on this computer.
    let broadcast = socket.send_to(DISCOVER, ("255.255.255.255", port));
    let local = socket.send_to(DISCOVER, ("127.0.0.1", port));
    broadcast.or(local).map_err(|e| e.to_string())?;
    let start = Instant::now();
    let mut rooms = HashMap::<u64, RoomEntry>::new();
    let mut buffer = [0; 512];
    while start.elapsed() < Duration::from_millis(1400) {
        if let Ok((len, source)) = socket.recv_from(&mut buffer) {
            if let Ok(reply) = serde_json::from_slice::<LanAnnouncement>(&buffer[..len]) {
                if reply.port == 0 || reply.players > 16 {
                    continue;
                }
                let address = format!("{}:{}", source.ip(), reply.port);
                if rooms.len() < 64
                    && (!source.ip().is_loopback() || !rooms.contains_key(&reply.id))
                {
                    rooms.insert(
                        reply.id,
                        RoomEntry {
                            code: String::new(),
                            name: clean(&reply.name, 12),
                            players: reply.players,
                            capacity: 16,
                            address,
                        },
                    );
                }
            }
        }
    }
    let mut rooms: Vec<_> = rooms.into_values().collect();
    rooms.sort_by(|a, b| a.address.cmp(&b.address));
    Ok(rooms)
}

#[cfg(test)]
pub fn discover_cloud(address: &str) -> Result<Vec<RoomEntry>, String> {
    discover_cloud_with_password(address, "")
}

#[cfg(test)]
pub fn discover_cloud_with_password(
    address: &str,
    password: &str,
) -> Result<Vec<RoomEntry>, String> {
    inspect_club(address, password, &AtomicBool::new(false)).map(|snapshot| snapshot.rooms)
}

#[derive(Debug, Clone)]
pub struct ClubSnapshot {
    pub rooms: Vec<RoomEntry>,
    pub latency_ms: Option<u32>,
}

pub fn inspect_club(
    address: &str,
    password: &str,
    stop: &AtomicBool,
) -> Result<ClubSnapshot, String> {
    let mut url = url_for(address, "", "", false)?;
    url.set_path("/lobby");
    url.set_query(Some("probe=1"));
    let mut socket = open_socket_authorized(&url, stop, password)?;
    #[derive(Deserialize)]
    struct Listing {
        rooms: Vec<RoomEntry>,
        #[serde(default)]
        probe: bool,
    }
    let message = socket.read().map_err(|e| e.to_string())?;
    let listing: Listing = serde_json::from_str(message.to_text().map_err(|e| e.to_string())?)
        .map_err(|_| "Server does not support the lobby".to_owned())?;
    let latency_ms = if listing.probe && !stop.load(Ordering::Relaxed) {
        match socket.get_mut() {
            MaybeTlsStream::Plain(stream) => stream.set_read_timeout(Some(Duration::from_secs(3))),
            MaybeTlsStream::NativeTls(stream) => stream
                .get_mut()
                .set_read_timeout(Some(Duration::from_secs(3))),
            _ => Err(io::Error::other("Unsupported connection type")),
        }
        .map_err(|error| error.to_string())?;
        let since = Instant::now();
        if socket.send(Message::text("ping")).is_ok()
            && matches!(socket.read(), Ok(Message::Text(text)) if text == "pong")
        {
            Some(since.elapsed().as_millis().clamp(1, u32::MAX as u128) as u32)
        } else {
            None
        }
    } else {
        None
    };
    let _ = socket.close(None);
    let rooms = listing
        .rooms
        .into_iter()
        .filter(|r| {
            r.code.len() == 8
                && r.code.bytes().all(|c| c.is_ascii_alphanumeric())
                && r.capacity <= 16
                && r.players <= 16
                && (r.capacity == 0 || r.players <= r.capacity)
        })
        .take(100)
        .map(|mut r| {
            r.name = clean(&r.name, 24);
            r.code.make_ascii_uppercase();
            r.address.clear();
            r
        })
        .collect();
    Ok(ClubSnapshot { rooms, latency_ms })
}

pub enum Mode {
    HostLan(u16),
    JoinLan(String),
    HostCloud { server: String, room_name: String },
    JoinCloud { server: String, room: String },
}

pub enum Event {
    Room { label: String, invite: String },
    Message(ServerMessage),
    Error(String),
}

pub struct Link {
    pub send: SyncSender<ClientMessage>,
    pub events: Mutex<Receiver<Event>>,
    stop: Arc<AtomicBool>,
}

impl Drop for Link {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}

#[cfg(test)]
pub fn start(mode: Mode, name: String) -> Link {
    start_with_options(mode, name, World::bundled(), String::new())
}

pub fn start_with_options(mode: Mode, name: String, world: World, password: String) -> Link {
    let (send, outgoing) = mpsc::sync_channel(64);
    let (incoming, events) = mpsc::sync_channel(256);
    let stop = Arc::new(AtomicBool::new(false));
    let thread_stop = stop.clone();
    thread::spawn(move || {
        if let Err(err) = client(
            mode,
            &name,
            outgoing,
            &incoming,
            thread_stop.clone(),
            world,
            &password,
        ) {
            let _ = incoming.try_send(Event::Error(err));
        }
        thread_stop.store(true, Ordering::Relaxed);
    });
    Link {
        send,
        events: Mutex::new(events),
        stop,
    }
}

fn url_for(address: &str, room: &str, name: &str, create: bool) -> Result<Url, String> {
    let address = normalize_server(address)?;
    let address = address.trim();
    let address = if address.contains("://") {
        address.to_owned()
    } else {
        format!("ws://{address}")
    };
    let mut url = Url::parse(&address).map_err(|_| "Invalid server address".to_owned())?;
    let scheme = match url.scheme() {
        "https" | "wss" => "wss",
        "http" | "ws" => "ws",
        _ => return Err("Use a ws:// or wss:// address".into()),
    };
    url.set_scheme(scheme).map_err(|_| "Invalid address")?;
    if url.host_str().is_none() || !url.username().is_empty() || url.password().is_some() {
        return Err("Enter a server address without credentials".into());
    }
    url.set_path(&format!("/room/{room}"));
    url.set_query(None);
    url.set_fragment(None);
    url.query_pairs_mut()
        .append_pair("name", name)
        .append_pair("create", if create { "1" } else { "0" })
        .append_pair("protocol", &PROTOCOL_VERSION.to_string());
    Ok(url)
}

pub fn normalize_server(address: &str) -> Result<String, String> {
    let address = address.trim();
    if address.is_empty() || address.len() > 256 {
        return Err("Enter a server address (up to 256 bytes)".into());
    }
    let address = if address.contains("://") {
        address.to_owned()
    } else {
        format!("ws://{address}")
    };
    let mut url = Url::parse(&address).map_err(|_| "Invalid server address")?;
    let scheme = match url.scheme() {
        "https" | "wss" => "wss",
        "http" | "ws" => "ws",
        _ => return Err("Use ws:// or wss://".into()),
    };
    url.set_scheme(scheme)
        .map_err(|_| "Invalid server address")?;
    if url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
        || !matches!(url.path(), "" | "/")
    {
        return Err("Use a server address without a path, credentials or query string".into());
    }
    Ok(url.to_string().trim_end_matches('/').to_owned())
}

/// Invitations identify both the server and its room, and never carry credentials.
pub fn room_invite(server: &str, room: &str) -> Result<String, String> {
    let (server, room) = room_target(server, room)?;
    Ok(format!("{server}/room/{room}"))
}

pub fn room_target(server: &str, invite: &str) -> Result<(String, String), String> {
    let invite = invite.trim();
    if invite.len() > 280 {
        return Err("Room invitation is too long".into());
    }
    let (server, room) = if invite.contains("://") {
        let mut url = Url::parse(invite).map_err(|_| "Invalid room invitation")?;
        let room = url
            .path()
            .strip_prefix("/room/")
            .ok_or("Use a room invitation ending in /room/ABCDEFGH")?
            .to_owned();
        url.set_path("");
        // normalize_server rejects credentials, query strings, fragments and
        // unsupported schemes, even when they arrive through a pasted invitation.
        (normalize_server(url.as_str())?, room)
    } else {
        (normalize_server(server)?, invite.to_owned())
    };
    let room = room.to_ascii_uppercase();
    if room.len() != 8 || !room.bytes().all(|c| c.is_ascii_alphanumeric()) {
        return Err("Enter an 8-character room code or a complete room invitation".into());
    }
    Ok((server, room))
}

fn config() -> WebSocketConfig {
    WebSocketConfig::default()
        .max_message_size(Some(MAX_WORLD_BYTES))
        .max_frame_size(Some(MAX_WORLD_BYTES))
}

fn client(
    mode: Mode,
    name: &str,
    outgoing: Receiver<ClientMessage>,
    events: &SyncSender<Event>,
    stop: Arc<AtomicBool>,
    world: World,
    password: &str,
) -> Result<(), String> {
    let installed_content = world.content.clone();
    let name = clean(name, 12);
    let name = if name.is_empty() { "Wanderer" } else { &name };
    let password = if matches!(mode, Mode::HostCloud { .. } | Mode::JoinCloud { .. }) {
        password
    } else {
        ""
    };
    let (url, label, invite) = match mode {
        Mode::HostLan(port) => {
            let listener = TcpListener::bind(("0.0.0.0", port))
                .map_err(|e| format!("Cannot host on port {port}: {e}"))?;
            let port = listener.local_addr().map_err(|e| e.to_string())?.port();
            // ponytail: one LAN host per computer; share discovery before supporting multiple hosts.
            let discovery = UdpSocket::bind(("0.0.0.0", DISCOVERY_PORT)).map_err(|e| {
                format!(
                    "LAN discovery port {DISCOVERY_PORT} is busy. Close the other local host: {e}"
                )
            })?;
            serve_world(listener, discovery, stop.clone(), name.to_owned(), world)
                .map_err(|e| e.to_string())?;
            let invite = format!("{}:{port}", lan_ip());
            (
                url_for(&format!("127.0.0.1:{port}"), "LOCAL", name, false)?,
                format!("LAN HOST / {invite}"),
                invite,
            )
        }
        Mode::JoinLan(address) => (
            url_for(&address, "LOCAL", name, false)?,
            format!("LAN / {address}"),
            address,
        ),
        Mode::HostCloud { server, room_name } => {
            let room_name = clean(&room_name, 24);
            if room_name.is_empty() {
                return Err("Please enter a room name.".into());
            }
            let room: String = rand::rng()
                .sample_iter(Alphanumeric)
                .take(8)
                .map(char::from)
                .collect::<String>()
                .to_uppercase();
            let mut url = url_for(&server, &room, name, true)?;
            url.query_pairs_mut().append_pair("room_name", &room_name);
            let invite = room_invite(&server, &room)?;
            (url, format!("{room_name} / {room}"), invite)
        }
        Mode::JoinCloud { server, room } => {
            let (server, room) = room_target(&server, &room)?;
            (
                url_for(&server, &room, name, false)?,
                format!("ROOM / {room}"),
                room_invite(&server, &room)?,
            )
        }
    };
    let mut socket = open_socket_authorized(&url, &stop, password)?;
    match socket.get_mut() {
        MaybeTlsStream::Plain(s) => s.set_read_timeout(Some(Duration::from_millis(10))),
        MaybeTlsStream::NativeTls(s) => s
            .get_mut()
            .set_read_timeout(Some(Duration::from_millis(10))),
        _ => Err(io::Error::other("Unsupported connection type")),
    }
    .map_err(|e| e.to_string())?;
    events
        .try_send(Event::Room { label, invite })
        .map_err(|e| e.to_string())?;
    let mut received_world = false;
    let mut welcomed = false;
    let mut heartbeat = Instant::now();
    let mut last_received = Instant::now();
    while !stop.load(Ordering::Relaxed) {
        for msg in outgoing.try_iter() {
            socket
                .send(Message::text(
                    serde_json::to_string(&msg).map_err(|e| e.to_string())?,
                ))
                .map_err(|e| e.to_string())?;
        }
        if heartbeat.elapsed() >= Duration::from_secs(15) {
            socket
                .send(Message::text("ping"))
                .map_err(|e| e.to_string())?;
            heartbeat = Instant::now();
        }
        match socket.read() {
            Ok(Message::Text(text)) => {
                last_received = Instant::now();
                if text != "pong" {
                    let msg: ServerMessage = serde_json::from_str(&text)
                        .map_err(|_| "Server protocol mismatch".to_owned())?;
                    match &msg {
                        ServerMessage::World { world } => {
                            if received_world || welcomed {
                                return Err("Server sent an unexpected map update".into());
                            }
                            world
                                .validate()
                                .map_err(|error| format!("Invalid server maps: {error}"))?;
                            world
                                .content
                                .require(&installed_content)
                                .map_err(|e| e.to_string())?;
                            socket
                                .send(Message::text(
                                    serde_json::to_string(&ClientMessage::WorldReady {
                                        revision: world.revision.clone(),
                                    })
                                    .unwrap(),
                                ))
                                .map_err(|e| e.to_string())?;
                            received_world = true;
                        }
                        ServerMessage::Welcome { players, .. } => {
                            if welcomed
                                || !received_world
                                || players.len() > 16
                                || players.iter().any(|p| {
                                    !installed_content
                                        .manifest
                                        .maps
                                        .iter()
                                        .any(|m| m.id == p.map)
                                })
                            {
                                return Err("Invalid room welcome".into());
                            }
                            welcomed = true;
                        }
                        _ if !welcomed => return Err("Room activity arrived before joining".into()),
                        _ => {}
                    }
                    events
                        .try_send(Event::Message(msg))
                        .map_err(|_| "Network queue full. Please reconnect.".to_owned())?;
                }
            }
            Ok(Message::Close(_)) => {
                return Err("Connection closed. Press Esc to return and rejoin.".into());
            }
            Ok(_) => {
                last_received = Instant::now();
            }
            Err(tungstenite::Error::Io(e))
                if matches!(
                    e.kind(),
                    io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut
                ) => {}
            Err(e) => return Err(format!("Disconnected: {e}")),
        }
        if last_received.elapsed() > Duration::from_secs(45) {
            return Err("Server timed out. Press Esc to return and rejoin.".into());
        }
    }
    let _ = socket.close(None);
    Ok(())
}

#[cfg(test)]
fn open_socket(
    url: &Url,
    stop: &AtomicBool,
) -> Result<WebSocket<MaybeTlsStream<TcpStream>>, String> {
    open_socket_authorized(url, stop, "")
}

fn open_socket_authorized(
    url: &Url,
    stop: &AtomicBool,
    password: &str,
) -> Result<WebSocket<MaybeTlsStream<TcpStream>>, String> {
    use tungstenite::client::IntoClientRequest;
    // Retry only before the WebSocket request: retrying a room creation could duplicate it.
    let mut stream = open_transport(url, stop);
    for _ in 0..2 {
        if stream.is_ok() || stop.load(Ordering::Relaxed) {
            break;
        }
        thread::sleep(Duration::from_millis(250));
        stream = open_transport(url, stop);
    }
    let mut request = url
        .as_str()
        .into_client_request()
        .map_err(|_| "Invalid server address")?;
    if !password.is_empty() {
        request.headers_mut().insert(
            "Authorization",
            yapshire_shared::protocol::authorization(password)
                .parse()
                .map_err(|_| "Invalid server password")?,
        );
    }
    let (socket, _) = tungstenite::client::client_with_config(request, stream?, Some(config()))
        .map_err(|e| {
            let detail = e.to_string();
            if detail.contains("401") {
                "Server password is missing or incorrect.".into()
            } else if detail.contains("426") {
                "Client and server use different map protocols. Update both to the same build."
                    .into()
            } else if detail.contains("403") {
                "Room creation is disabled. Join the server's existing town.".into()
            } else if detail.contains("404") {
                "Room not found, or everyone has left.".into()
            } else if detail.contains("409") {
                "Room is full or the code is taken. Try again.".into()
            } else {
                format!("WebSocket connection failed: {detail}")
            }
        })?;
    Ok(socket)
}

fn open_transport(url: &Url, stop: &AtomicBool) -> Result<MaybeTlsStream<TcpStream>, String> {
    let host = url.host_str().ok_or("Missing hostname")?;
    let port = url.port_or_known_default().ok_or("Missing port")?;
    let proxy = if url.scheme() == "wss" && !bypass_proxy(host) {
        ["https_proxy", "HTTPS_PROXY", "all_proxy", "ALL_PROXY"]
            .iter()
            .find_map(|key| std::env::var(key).ok().filter(|v| !v.is_empty()))
            .map(|value| Url::parse(&value).map_err(|_| "Invalid HTTPS proxy configuration"))
            .transpose()?
    } else {
        None
    };
    if let Some(proxy) = &proxy {
        if proxy.scheme() != "http" || !proxy.username().is_empty() || proxy.password().is_some() {
            return Err(
                "This client supports unauthenticated HTTP CONNECT proxies. Check HTTPS_PROXY."
                    .into(),
            );
        }
    }
    let endpoint = proxy.as_ref().map_or((host, port), |p| {
        (
            p.host_str().unwrap_or("localhost"),
            p.port_or_known_default().unwrap_or(80),
        )
    });
    let addresses: Vec<SocketAddr> = endpoint
        .to_socket_addrs()
        .map_err(|e| format!("Cannot resolve server: {e}"))?
        .collect();
    let mut stream = None;
    for address in addresses {
        if stop.load(Ordering::Relaxed) {
            return Err("Connection cancelled".into());
        }
        if let Ok(s) = TcpStream::connect_timeout(&address, Duration::from_secs(4)) {
            stream = Some(s);
            break;
        }
    }
    let mut stream = stream.ok_or("Cannot connect. Check the address, host and firewall.")?;
    stream
        .set_read_timeout(Some(Duration::from_secs(12)))
        .map_err(|e| e.to_string())?;
    stream
        .set_write_timeout(Some(Duration::from_secs(3)))
        .map_err(|e| e.to_string())?;
    stream.set_nodelay(true).map_err(|e| e.to_string())?;
    if proxy.is_some() {
        write!(
            stream,
            "CONNECT {host}:{port} HTTP/1.1\r\nHost: {host}:{port}\r\n\r\n"
        )
        .map_err(|e| e.to_string())?;
        let mut headers = Vec::new();
        while !headers.ends_with(b"\r\n\r\n") {
            if headers.len() >= 8192 {
                return Err("Proxy response is too large".into());
            }
            let mut byte = [0];
            stream
                .read_exact(&mut byte)
                .map_err(|e| format!("Proxy failed: {e}"))?;
            headers.push(byte[0]);
        }
        if !String::from_utf8_lossy(&headers)
            .lines()
            .next()
            .unwrap_or("")
            .split_whitespace()
            .nth(1)
            .is_some_and(|s| s == "200")
        {
            return Err("HTTPS proxy rejected the connection".into());
        }
    }
    // Handle timed-out TLS handshakes here; tungstenite's native-TLS helper panics on WouldBlock.
    let stream = if url.scheme() == "wss" {
        let tls = native_tls::TlsConnector::new().map_err(|e| e.to_string())?;
        MaybeTlsStream::NativeTls(
            tls.connect(host, stream)
                .map_err(|e| format!("TLS connection failed; try refreshing: {e}"))?,
        )
    } else {
        MaybeTlsStream::Plain(stream)
    };
    Ok(stream)
}

fn bypass_proxy(host: &str) -> bool {
    if host == "localhost"
        || host
            .parse::<std::net::IpAddr>()
            .is_ok_and(|ip| ip.is_loopback())
    {
        return true;
    }
    std::env::var("no_proxy")
        .or_else(|_| std::env::var("NO_PROXY"))
        .unwrap_or_default()
        .split(',')
        .any(|entry| {
            let entry = entry.trim().trim_start_matches('.');
            !entry.is_empty()
                && (entry == "*" || host == entry || host.ends_with(&format!(".{entry}")))
        })
}

pub fn lan_ip() -> String {
    UdpSocket::bind("0.0.0.0:0")
        .and_then(|socket| {
            socket.connect("192.0.2.1:9")?;
            socket.local_addr()
        })
        .map(|addr| addr.ip().to_string())
        .unwrap_or_else(|_| "127.0.0.1".into())
}

#[cfg(test)]
pub fn serve(
    listener: TcpListener,
    discovery: UdpSocket,
    stop: Arc<AtomicBool>,
    owner: String,
) -> io::Result<()> {
    serve_world(listener, discovery, stop, owner, World::bundled())
}

fn serve_world(
    listener: TcpListener,
    discovery: UdpSocket,
    stop: Arc<AtomicBool>,
    owner: String,
    world: World,
) -> io::Result<()> {
    discovery.set_nonblocking(true)?;
    let port = listener.local_addr()?.port();
    let instance = rand::random::<u64>();
    let server =
        yapshire_server::Server::new(yapshire_server::Config::lan(owner.clone()), world, "")?;
    server.clone().spawn(listener, stop.clone())?;
    thread::spawn(move || {
        while !stop.load(Ordering::Relaxed) {
            let mut buffer = [0; 64];
            for _ in 0..16 {
                let Ok((len, source)) = discovery.recv_from(&mut buffer) else {
                    break;
                };
                if &buffer[..len] == DISCOVER {
                    let room = LanAnnouncement {
                        id: instance,
                        name: owner.clone(),
                        players: server.player_count("LOCAL"),
                        port,
                    };
                    if let Ok(bytes) = serde_json::to_vec(&room) {
                        let _ = discovery.send_to(&bytes, source);
                    }
                }
            }
            thread::sleep(Duration::from_millis(10));
        }
    });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn club_refresh_measures_a_round_trip_without_joining_and_reports_actual_capacity() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = format!("ws://{}", listener.local_addr().unwrap());
        let server = yapshire_server::Server::new(
            yapshire_server::Config {
                max_players: 3,
                ..Default::default()
            },
            World::bundled(),
            "private-club",
        )
        .unwrap();
        let stop = Arc::new(AtomicBool::new(false));
        let thread = server.clone().spawn(listener, stop.clone()).unwrap();
        let snapshot = inspect_club(&address, "private-club", &AtomicBool::new(false)).unwrap();
        assert_eq!(snapshot.rooms.len(), 1);
        assert_eq!(snapshot.rooms[0].capacity, 3);
        assert_eq!(snapshot.rooms[0].players, 0);
        assert!(snapshot.latency_ms.is_some_and(|ms| ms > 0));
        assert_eq!(server.player_count("MAIN0001"), 0);
        assert!(inspect_club(&address, "wrong-password", &AtomicBool::new(false)).is_err());
        stop.store(true, Ordering::Relaxed);
        thread.join().unwrap();
    }

    #[test]
    fn missing_content_is_rejected_before_acknowledgement_or_player_creation() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = format!("ws://{}", listener.local_addr().unwrap());
        let mut world = World::bundled();
        world
            .content
            .images
            .insert("objects/harbor.png".into(), "1".repeat(64));
        let world = World::from_maps(world.content, world.maps).unwrap();
        let server = yapshire_server::Server::new(Default::default(), world, "").unwrap();
        let stop = Arc::new(AtomicBool::new(false));
        let thread = server.clone().spawn(listener, stop.clone()).unwrap();
        let client = start(
            Mode::JoinCloud {
                server: address,
                room: "MAIN0001".into(),
            },
            "Guest".into(),
        );
        let error = loop {
            match client
                .events
                .lock()
                .unwrap()
                .recv_timeout(Duration::from_secs(5))
                .unwrap()
            {
                Event::Error(e) => break e,
                Event::Message(ServerMessage::Welcome { .. }) => {
                    panic!("Mismatched client entered the world")
                }
                _ => {}
            }
        };
        assert!(error.contains("objects/harbor.png") && error.contains("Required pack"));
        assert_eq!(server.player_count("MAIN0001"), 0);
        drop(client);
        stop.store(true, Ordering::Relaxed);
        thread.join().unwrap();
    }

    #[test]
    fn old_clubs_still_list_rooms_without_inventing_ping_or_capacity() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = format!("ws://{}", listener.local_addr().unwrap());
        let server = thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            let mut socket = tungstenite::accept(stream).unwrap();
            socket.send(Message::text(r#"{"rooms":[{"code":"LEGACY01","name":"Old town","players":2,"address":"wss://redirect.example"}]}"#)).unwrap();
            socket.close(None).unwrap();
        });
        let snapshot = inspect_club(&address, "", &AtomicBool::new(false)).unwrap();
        assert_eq!(snapshot.rooms[0].players, 2);
        assert_eq!(snapshot.rooms[0].capacity, 0);
        assert!(snapshot.rooms[0].address.is_empty());
        assert!(snapshot.latency_ms.is_none());
        server.join().unwrap();
    }

    #[test]
    fn stalled_tls_returns_an_error_instead_of_panicking() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let (release, hold) = mpsc::channel::<()>();
        let server = thread::spawn(move || {
            let (_stream, _) = listener.accept().unwrap();
            let _ = hold.recv_timeout(Duration::from_secs(20));
        });
        let url = Url::parse(&format!("wss://{address}/room/ABCDEFGH")).unwrap();
        let result = open_transport(&url, &AtomicBool::new(false));
        let _ = release.send(());
        server.join().unwrap();
        assert!(matches!(result, Err(error) if error.contains("TLS connection failed")));
    }

    #[test]
    fn broken_tls_is_retried_at_most_three_times() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = thread::spawn(move || {
            for _ in 0..3 {
                let (stream, _) = listener.accept().unwrap();
                drop(stream);
            }
        });
        let url = Url::parse(&format!("wss://{address}/room/ABCDEFGH")).unwrap();
        assert!(open_socket(&url, &AtomicBool::new(false)).is_err());
        server.join().unwrap();
    }

    #[test]
    #[ignore = "requires YAPSHIRE_TEST_SERVER pointing to a running Worker"]
    fn cloud_client_hosts_discovers_joins_moves_and_chats() {
        let server = std::env::var("YAPSHIRE_TEST_SERVER").unwrap();
        fn welcome(link: &Link) -> (u32, String) {
            let mut code = String::new();
            loop {
                match link
                    .events
                    .lock()
                    .unwrap()
                    .recv_timeout(Duration::from_secs(15))
                    .unwrap()
                {
                    Event::Room { invite, .. } => code = invite,
                    Event::Message(ServerMessage::Welcome { you, .. }) => return (you, code),
                    Event::Error(error) => panic!("{error}"),
                    _ => {}
                }
            }
        }
        fn next(link: &Link, kind: &str) -> ServerMessage {
            let deadline = Instant::now() + Duration::from_secs(15);
            while Instant::now() < deadline {
                match link
                    .events
                    .lock()
                    .unwrap()
                    .recv_timeout(Duration::from_secs(5))
                    .unwrap()
                {
                    Event::Message(msg)
                        if matches!(
                            (&msg, kind),
                            (ServerMessage::Moved { .. }, "move")
                                | (ServerMessage::Chat { .. }, "chat")
                                | (ServerMessage::Left { .. }, "left")
                        ) =>
                    {
                        return msg;
                    }
                    Event::Error(error) => panic!("{error}"),
                    _ => {}
                }
            }
            panic!("Missing {kind}");
        }
        let a = start(
            Mode::HostCloud {
                server: server.clone(),
                room_name: "A quiet afternoon".into(),
            },
            "Rust Host".into(),
        );
        let (id, invite) = welcome(&a);
        let (_, code) = room_target(&server, &invite).unwrap();
        let listing = discover_cloud(&server).unwrap();
        assert!(
            listing
                .iter()
                .any(|r| r.code == code && r.players == 1 && r.name == "A quiet afternoon")
        );
        let b = start(
            Mode::JoinCloud {
                server: server.clone(),
                room: invite,
            },
            "Rust Guest".into(),
        );
        let (other, _) = welcome(&b);
        assert_ne!(id, other);
        a.send
            .send(ClientMessage::Move {
                map: "yapshire:tackle_shop".into(),
                x: 400.0,
                y: 20.0,
                moving: true,
                facing: false,
                indoors: true,
                fishing: false,
            })
            .unwrap();
        assert!(
            matches!(next(&b, "move"), ServerMessage::Moved { player } if player.id == id && player.x == 400.0 && player.indoors && !player.fishing)
        );
        a.send
            .send(ClientMessage::Chat {
                text: "Hello from Rust!".into(),
            })
            .unwrap();
        assert!(
            matches!(next(&b, "chat"), ServerMessage::Chat { id: speaker, text } if speaker == id && text == "Hello from Rust!")
        );
        drop(a);
        assert!(matches!(next(&b, "left"), ServerMessage::Left { id: left } if left == id));
    }

    #[test]
    fn invitations_round_trip_the_destination_without_credentials() {
        let invite = room_invite("https://friends.example:8443/", "niannian").unwrap();
        assert_eq!(invite, "wss://friends.example:8443/room/NIANNIAN");
        assert_eq!(
            room_target("ws://wrong.example", &invite).unwrap(),
            ("wss://friends.example:8443".into(), "NIANNIAN".into())
        );
        assert_eq!(
            room_target("http://[::1]:4761", "abcdefgh").unwrap(),
            ("ws://[::1]:4761".into(), "ABCDEFGH".into())
        );
        for invite in [
            "wss://user:secret@friends.example/room/NIANNIAN",
            "wss://friends.example/room/NIANNIAN?password=secret",
            "wss://friends.example/room/NIANNIAN#create=1",
            "wss://friends.example/room/NIANNIAN/extra",
            "file:///room/NIANNIAN",
            "wss://friends.example/room/短房间码",
            "NIANNIAN?create=1",
        ] {
            assert!(
                room_target(DEFAULT_SERVER.trim(), invite).is_err(),
                "{invite}"
            );
        }
    }

    #[test]
    fn text_and_addresses_are_bounded() {
        assert_eq!(clean("\n 你好\u{202e}\0 world \t", 5), "你好 w");
        assert_eq!(clean(&"中".repeat(200), 80).chars().count(), 80);
        let url = url_for("https://example.com/", "ABCDEFGH", "小风&雨", true).unwrap();
        assert_eq!(url.scheme(), "wss");
        assert_eq!(url.path(), "/room/ABCDEFGH");
        assert!(url.as_str().contains("%26"));
        assert!(url_for("file:///etc/passwd", "ABCD", "x", false).is_err());
    }

    #[test]
    fn two_real_websockets_sync_move_chat_and_disconnect() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let stop = Arc::new(AtomicBool::new(false));
        let discovery = UdpSocket::bind("0.0.0.0:0").unwrap();
        let discovery_port = discovery.local_addr().unwrap().port();
        serve(listener, discovery, stop.clone(), "Test Host".into()).unwrap();
        assert!(
            scan_lan(discovery_port)
                .unwrap()
                .iter()
                .any(|r| r.name == "Test Host" && r.address.ends_with(&addr.port().to_string()))
        );
        let connect = |name| {
            let url = url_for(&addr.to_string(), "LOCAL", name, false).unwrap();
            let (mut ws, _) = tungstenite::connect(url.as_str()).unwrap();
            if let MaybeTlsStream::Plain(s) = ws.get_mut() {
                s.set_read_timeout(Some(Duration::from_secs(3))).unwrap();
            }
            let world: ServerMessage =
                serde_json::from_str(ws.read().unwrap().to_text().unwrap()).unwrap();
            let ServerMessage::World { world } = world else {
                panic!("Expected map before welcome")
            };
            world.validate().unwrap();
            ws.send(Message::text(
                serde_json::to_string(&ClientMessage::WorldReady {
                    revision: world.revision,
                })
                .unwrap(),
            ))
            .unwrap();
            ws
        };
        let read = |ws: &mut WebSocket<MaybeTlsStream<TcpStream>>| -> ServerMessage {
            serde_json::from_str(ws.read().unwrap().to_text().unwrap()).unwrap()
        };
        let mut a = connect("小风");
        let ServerMessage::Welcome { you, .. } = read(&mut a) else {
            panic!("welcome")
        };
        let mut b = connect("小雨");
        assert!(
            matches!(read(&mut b), ServerMessage::Welcome { players, .. } if players.len() == 2)
        );
        assert!(matches!(read(&mut a), ServerMessage::Joined { .. }));
        a.send(Message::text(
            r#"{"type":"move","map":"yapshire:town","x":700,"y":12,"moving":true,"facing":true}"#,
        ))
        .unwrap();
        assert!(
            matches!(read(&mut b), ServerMessage::Moved { player } if player.id == you && player.x == 700.0 && player.facing)
        );
        a.send(Message::text(
            r#"{"type":"move","map":"yapshire:tackle_shop","x":270,"y":0,"moving":false,"facing":false,"indoors":true,"fishing":true}"#,
        )).unwrap();
        assert!(
            matches!(read(&mut b), ServerMessage::Moved { player } if player.indoors && !player.fishing)
        );
        a.send(Message::text(
            r#"{"type":"move","map":"yapshire:town","x":1320,"y":0,"moving":false,"facing":false,"indoors":false,"fishing":true}"#,
        )).unwrap();
        assert!(
            matches!(read(&mut b), ServerMessage::Moved { player } if !player.indoors && player.fishing)
        );
        a.send(Message::text(r#"{"type":"chat","text":"你好，小镇！"}"#))
            .unwrap();
        assert!(matches!(read(&mut a), ServerMessage::Chat { text, .. } if text == "你好，小镇！"));
        assert!(matches!(read(&mut b), ServerMessage::Chat { id, .. } if id == you));
        a.close(None).unwrap();
        assert!(matches!(read(&mut b), ServerMessage::Left { id } if id == you));
        stop.store(true, Ordering::Relaxed);
    }
}
