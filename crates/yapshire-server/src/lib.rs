//! The same bounded room server runs headlessly and inside a LAN host.
mod config;
use axum::{
    Json, Router,
    extract::{
        ConnectInfo, Path, Query, State, WebSocketUpgrade,
        ws::{CloseFrame, Message, WebSocket},
    },
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
};
pub use config::Config;
use serde::Deserialize;
use std::{
    collections::HashMap,
    future::Future,
    io,
    net::{IpAddr, SocketAddr},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};
use subtle::ConstantTimeEq;
use tokio::sync::{OwnedSemaphorePermit, Semaphore, mpsc, watch};
use yapshire_shared::{
    PROTOCOL_VERSION, World,
    protocol::{ClientMessage, Player, RoomEntry, ServerMessage, authorization, clean},
};

#[derive(Clone)]
pub struct Server(Arc<Inner>);
struct Inner {
    config: Config,
    world: World,
    world_message: String,
    authorization: String,
    rooms: Mutex<HashMap<String, Room>>,
    requests: Mutex<HashMap<IpAddr, (Instant, usize)>>,
    connections: Mutex<HashMap<IpAddr, usize>>,
    slots: Arc<Semaphore>,
    shutdown: watch::Sender<bool>,
}
struct Room {
    name: String,
    persistent: bool,
    peers: HashMap<u32, Peer>,
}
struct Peer {
    player: Player,
    send: mpsc::Sender<Outgoing>,
    movements: Arc<Mutex<HashMap<u32, Player>>>,
    stop: watch::Sender<bool>,
}
enum Outgoing {
    Message(ServerMessage),
    Movement(u32),
}
type Failure = (StatusCode, &'static str);

impl Server {
    pub fn new(config: Config, world: World, password: &str) -> io::Result<Self> {
        config.validate()?;
        world.validate()?;
        if !password.is_empty() && !(8..=128).contains(&password.len()) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Server password must contain 8 to 128 bytes",
            ));
        }
        let rooms = HashMap::from([(
            config.room_code.clone(),
            Room {
                name: clean(&config.name, 24),
                persistent: true,
                peers: HashMap::new(),
            },
        )]);
        let world_message = serde_json::to_string(&ServerMessage::World {
            world: Box::new(world.clone()),
        })?;
        let capacity = config.max_rooms * config.max_players + 8;
        let (shutdown, _) = watch::channel(false);
        Ok(Self(Arc::new(Inner {
            config,
            world,
            world_message,
            authorization: authorization(password),
            rooms: Mutex::new(rooms),
            requests: Mutex::default(),
            connections: Mutex::default(),
            slots: Arc::new(Semaphore::new(capacity)),
            shutdown,
        })))
    }

    pub fn player_count(&self, code: &str) -> usize {
        self.0
            .rooms
            .lock()
            .unwrap()
            .get(code)
            .map_or(0, |room| room.peers.len())
    }

    pub fn revision(&self) -> &str {
        &self.0.world.revision
    }

    pub async fn run(
        self,
        listener: std::net::TcpListener,
        shutdown: impl Future<Output = ()> + Send + 'static,
    ) -> io::Result<()> {
        listener.set_nonblocking(true)?;
        let listener = tokio::net::TcpListener::from_std(listener)?;
        let router = Router::new()
            .route("/health", get(health))
            .route("/rooms", get(rooms))
            .route("/lobby", get(lobby))
            .route("/room/{code}", get(room))
            .with_state(self.clone());
        let stopping = self.clone();
        let result = axum::serve(
            listener,
            router.into_make_service_with_connect_info::<SocketAddr>(),
        )
        .with_graceful_shutdown(async move {
            shutdown.await;
            stopping.0.shutdown.send_replace(true);
        })
        .await;
        // Upgraded sockets outlive Axum's HTTP connections. Let their close
        // frames and participant cleanup finish before dropping the runtime.
        self.0.shutdown.send_replace(true);
        let _ = tokio::time::timeout(Duration::from_secs(7), async {
            while !self.0.connections.lock().unwrap().is_empty() {
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await;
        result
    }

    pub fn spawn(
        self,
        listener: std::net::TcpListener,
        stop: Arc<AtomicBool>,
    ) -> io::Result<std::thread::JoinHandle<()>> {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()?;
        Ok(std::thread::spawn(move || {
            let _ = runtime.block_on(self.run(listener, async move {
                while !stop.load(Ordering::Relaxed) {
                    tokio::time::sleep(Duration::from_millis(50)).await;
                }
            }));
        }))
    }

    fn authorize(&self, ip: IpAddr, headers: &HeaderMap) -> Result<(), Failure> {
        let now = Instant::now();
        let mut requests = self.0.requests.lock().unwrap();
        if requests.len() >= 4096 {
            requests.retain(|_, (since, _)| since.elapsed() < Duration::from_secs(60));
        }
        if requests.len() >= 4096 && !requests.contains_key(&ip) {
            return Err((
                StatusCode::TOO_MANY_REQUESTS,
                "Server request capacity reached",
            ));
        }
        let budget = requests.entry(ip).or_insert((now, 0));
        if budget.0.elapsed() >= Duration::from_secs(60) {
            *budget = (now, 0);
        }
        budget.1 += 1;
        if budget.1 > 300 {
            return Err((
                StatusCode::TOO_MANY_REQUESTS,
                "Too many connection attempts",
            ));
        }
        drop(requests);
        if let Some(origin) = headers.get("origin") {
            if !origin.to_str().ok().is_some_and(|value| {
                self.0
                    .config
                    .allowed_origins
                    .iter()
                    .any(|allowed| allowed == value)
            }) {
                return Err((StatusCode::FORBIDDEN, "Browser origin is not allowed"));
            }
        }
        if !self.0.authorization.is_empty() {
            let provided = headers
                .get("authorization")
                .map_or(&[][..], |h| h.as_bytes());
            if !bool::from(provided.ct_eq(self.0.authorization.as_bytes())) {
                return Err((
                    StatusCode::UNAUTHORIZED,
                    "Server password is missing or incorrect",
                ));
            }
        }
        Ok(())
    }

    fn reserve(&self, ip: IpAddr) -> Result<Connection, Failure> {
        let slot = self.0.slots.clone().try_acquire_owned().map_err(|_| {
            (
                StatusCode::SERVICE_UNAVAILABLE,
                "Server connection capacity reached",
            )
        })?;
        let mut connections = self.0.connections.lock().unwrap();
        let count = connections.entry(ip).or_default();
        if *count >= self.0.config.max_connections_per_ip {
            return Err((
                StatusCode::TOO_MANY_REQUESTS,
                "Too many connections from this address",
            ));
        }
        *count += 1;
        Ok(Connection {
            server: self.clone(),
            ip,
            _slot: slot,
        })
    }

    fn listing(&self) -> Vec<RoomEntry> {
        let mut rooms: Vec<_> = self
            .0
            .rooms
            .lock()
            .unwrap()
            .iter()
            .map(|(code, room)| RoomEntry {
                code: code.clone(),
                name: room.name.clone(),
                players: room.peers.len(),
                capacity: self.0.config.max_players,
                address: String::new(),
            })
            .collect();
        rooms.sort_by(|a, b| a.code.cmp(&b.code));
        rooms
    }
}

struct Connection {
    server: Server,
    ip: IpAddr,
    _slot: OwnedSemaphorePermit,
}
impl Drop for Connection {
    fn drop(&mut self) {
        let mut connections = self.server.0.connections.lock().unwrap();
        if let Some(count) = connections.get_mut(&self.ip) {
            *count -= 1;
            if *count == 0 {
                connections.remove(&self.ip);
            }
        }
    }
}
struct Participant {
    server: Server,
    code: String,
    id: u32,
}
impl Drop for Participant {
    fn drop(&mut self) {
        let mut rooms = self.server.0.rooms.lock().unwrap();
        if let Some(room) = rooms.get_mut(&self.code) {
            room.peers.remove(&self.id);
            broadcast(room, ServerMessage::Left { id: self.id }, None);
            if room.peers.is_empty() && !room.persistent {
                rooms.remove(&self.code);
            }
        }
    }
}

async fn health(State(server): State<Server>) -> Json<serde_json::Value> {
    Json(
        serde_json::json!({"ok": true, "game": "yapshire", "version": env!("CARGO_PKG_VERSION"), "protocol": PROTOCOL_VERSION, "world": server.revision()}),
    )
}
async fn rooms(
    State(server): State<Server>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, Failure> {
    server.authorize(addr.ip(), &headers)?;
    Ok(Json(serde_json::json!({"rooms": server.listing()})))
}
#[derive(Default, Deserialize)]
struct LobbyQuery {
    #[serde(default)]
    probe: u8,
}

async fn lobby(
    State(server): State<Server>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Query(query): Query<LobbyQuery>,
    headers: HeaderMap,
    ws: WebSocketUpgrade,
) -> Result<Response, Failure> {
    server.authorize(addr.ip(), &headers)?;
    let connection = server.reserve(addr.ip())?;
    let probe = query.probe == 1;
    let listing = serde_json::json!({"rooms": server.listing(), "probe": probe}).to_string();
    Ok(ws
        .max_message_size(2048)
        .max_frame_size(2048)
        .on_upgrade(move |mut socket| async move {
            let _connection = connection;
            let _ = send(&mut socket, Message::Text(listing.into())).await;
            // A single, bounded round trip measures the game's transport without
            // joining a room, sending maps or changing any player counts.
            if probe {
                if let Ok(Some(Ok(Message::Text(text)))) =
                    tokio::time::timeout(Duration::from_secs(3), socket.recv()).await
                {
                    if text == "ping" {
                        let _ = send(&mut socket, Message::Text("pong".into())).await;
                    }
                }
            }
            close(&mut socket, 1000, "Lobby listed").await;
        }))
}

#[derive(Deserialize)]
struct Join {
    #[serde(default)]
    protocol: u32,
    #[serde(default)]
    create: u8,
    #[serde(default)]
    name: String,
    #[serde(default)]
    room_name: String,
}
async fn room(
    State(server): State<Server>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Path(code): Path<String>,
    Query(join): Query<Join>,
    headers: HeaderMap,
    ws: WebSocketUpgrade,
) -> Result<Response, Failure> {
    server.authorize(addr.ip(), &headers)?;
    if join.protocol != PROTOCOL_VERSION {
        return Err((
            StatusCode::UPGRADE_REQUIRED,
            "Client and server map protocols differ. Use matching Yapshire builds.",
        ));
    }
    if !(config::valid_code(&code) || code == server.0.config.room_code)
        || join.name.len() > 128
        || join.room_name.len() > 256
        || join.create > 1
    {
        return Err((StatusCode::BAD_REQUEST, "Invalid room or player name"));
    }
    {
        let rooms = server.0.rooms.lock().unwrap();
        if join.create == 1 {
            if !server.0.config.allow_room_creation {
                return Err((
                    StatusCode::FORBIDDEN,
                    "Room creation is disabled; join the existing town",
                ));
            }
            if clean(&join.room_name, 24).is_empty() {
                return Err((StatusCode::BAD_REQUEST, "Room name required"));
            }
            if rooms.contains_key(&code) {
                return Err((StatusCode::CONFLICT, "Room already exists"));
            }
            if rooms.len() >= server.0.config.max_rooms {
                return Err((
                    StatusCode::SERVICE_UNAVAILABLE,
                    "Server room capacity reached",
                ));
            }
        } else {
            let room = rooms
                .get(&code)
                .ok_or((StatusCode::NOT_FOUND, "Room not found"))?;
            if room.peers.len() >= server.0.config.max_players {
                return Err((StatusCode::CONFLICT, "Room is full"));
            }
        }
    }
    let connection = server.reserve(addr.ip())?;
    Ok(ws
        .max_message_size(2048)
        .max_frame_size(2048)
        .on_upgrade(move |socket| session(socket, server, code, join, connection))
        .into_response())
}

fn broadcast(room: &Room, message: ServerMessage, except: Option<u32>) {
    for (id, peer) in &room.peers {
        if Some(*id) == except {
            continue;
        }
        let outgoing = match &message {
            ServerMessage::Moved { player } => {
                // Keep one queued position per player. A brief transport stall
                // must not fill the reliable queue with obsolete movement frames.
                let mut pending = peer.movements.lock().unwrap();
                if pending.insert(player.id, player.clone()).is_some() {
                    continue;
                }
                Outgoing::Movement(player.id)
            }
            _ => Outgoing::Message(message.clone()),
        };
        if peer.send.try_send(outgoing).is_err() {
            let _ = peer.stop.send(true);
        }
    }
}
async fn send(socket: &mut WebSocket, message: Message) -> bool {
    matches!(
        tokio::time::timeout(Duration::from_secs(3), socket.send(message)).await,
        Ok(Ok(()))
    )
}
async fn close(socket: &mut WebSocket, code: u16, reason: &'static str) {
    let _ = send(
        socket,
        Message::Close(Some(CloseFrame {
            code,
            reason: reason.into(),
        })),
    )
    .await;
}

async fn session(
    mut socket: WebSocket,
    server: Server,
    code: String,
    join: Join,
    _connection: Connection,
) {
    let mut shutdown = server.0.shutdown.subscribe();
    if *shutdown.borrow() {
        close(&mut socket, 1001, "Server shutting down").await;
        return;
    }
    if !send(
        &mut socket,
        Message::Text(server.0.world_message.clone().into()),
    )
    .await
    {
        return;
    }
    let first = tokio::select! {
        first = tokio::time::timeout(Duration::from_secs(10), socket.recv()) => first,
        _ = shutdown.changed() => {
            close(&mut socket, 1001, "Server shutting down").await;
            return;
        }
    };
    let acknowledged = match first {
        Ok(Some(Ok(Message::Text(raw)))) => {
            matches!(serde_json::from_str::<ClientMessage>(&raw), Ok(ClientMessage::WorldReady { revision }) if revision == server.revision())
        }
        _ => false,
    };
    if !acknowledged {
        close(
            &mut socket,
            1008,
            "Map revision must be acknowledged before joining",
        )
        .await;
        return;
    }
    let (tx, mut rx) = mpsc::channel(128);
    let movements = Arc::new(Mutex::new(HashMap::new()));
    let (stop, mut stopped) = watch::channel(false);
    let joined = {
        let mut rooms = server.0.rooms.lock().unwrap();
        if join.create == 1
            && (rooms.contains_key(&code) || rooms.len() >= server.0.config.max_rooms)
        {
            None
        } else {
            if join.create == 1 {
                rooms.insert(
                    code.clone(),
                    Room {
                        name: clean(&join.room_name, 24),
                        persistent: false,
                        peers: HashMap::new(),
                    },
                );
            }
            rooms
                .get_mut(&code)
                .filter(|room| room.peers.len() < server.0.config.max_players)
                .map(|room| {
                    let id = loop {
                        let id = rand::random::<u32>();
                        if !room.peers.contains_key(&id) {
                            break id;
                        }
                    };
                    let name = clean(&join.name, 12);
                    let player = Player {
                        id,
                        name: if name.is_empty() {
                            "Wanderer".into()
                        } else {
                            name
                        },
                        map: server.0.world.entry().0.to_owned(),
                        x: server.0.world.entry().1[0],
                        y: server.0.world.entry().1[1],
                        moving: false,
                        facing: false,
                        indoors: server
                            .0
                            .world
                            .content
                            .manifest
                            .maps
                            .iter()
                            .find(|m| m.id == server.0.world.entry().0)
                            .unwrap()
                            .indoors,
                        fishing: false,
                    };
                    broadcast(
                        room,
                        ServerMessage::Joined {
                            player: player.clone(),
                        },
                        None,
                    );
                    room.peers.insert(
                        id,
                        Peer {
                            player,
                            send: tx.clone(),
                            movements: movements.clone(),
                            stop,
                        },
                    );
                    let _ = tx.try_send(Outgoing::Message(ServerMessage::Welcome {
                        you: id,
                        players: room.peers.values().map(|p| p.player.clone()).collect(),
                    }));
                    Participant {
                        server: server.clone(),
                        code: code.clone(),
                        id,
                    }
                })
        }
    };
    let Some(participant) = joined else {
        close(
            &mut socket,
            1008,
            "Room became unavailable; refresh and try again",
        )
        .await;
        return;
    };
    let mut timer = tokio::time::interval(Duration::from_secs(1));
    let mut last_seen = Instant::now();
    let mut last_chat = Instant::now() - Duration::from_secs(1);
    let mut rate = (Instant::now(), 0);
    let (mut close_code, mut reason) = (1000, "Left room");
    loop {
        tokio::select! {
            _ = shutdown.changed() => { reason = "Server shutting down"; break; }
            _ = stopped.changed() => { close_code = 1013; reason = "Client is too slow"; break; }
            _ = timer.tick() => {
                if last_seen.elapsed() > Duration::from_secs(45) { close_code = 1001; reason = "Heartbeat timed out"; break; }
            }
            Some(message) = rx.recv() => {
                let message = match message {
                    Outgoing::Message(message) => message,
                    Outgoing::Movement(id) => {
                        let Some(player) = movements.lock().unwrap().remove(&id) else { continue; };
                        ServerMessage::Moved { player }
                    }
                };
                if !send(&mut socket, Message::Text(serde_json::to_string(&message).unwrap().into())).await { break; }
            }
            message = socket.recv() => {
                let message = match message {
                    Some(Ok(message)) => message,
                    Some(Err(_)) => { close_code = 1009; reason = "Invalid or oversized WebSocket message"; break; }
                    None => break,
                };
                last_seen = Instant::now();
                if rate.0.elapsed() >= Duration::from_secs(1) { rate = (Instant::now(), 0); }
                rate.1 += 1;
                if rate.1 > 80 { close_code = 1008; reason = "Too many messages"; break; }
                let raw = match message {
                    Message::Text(raw) => raw,
                    Message::Close(_) => break,
                    Message::Ping(_) | Message::Pong(_) => continue,
                    Message::Binary(_) => { close_code = 1003; reason = "Text messages required"; break; }
                };
                if raw == "ping" { if !send(&mut socket, Message::Text("pong".into())).await { break; } continue; }
                let Ok(message) = serde_json::from_str::<ClientMessage>(&raw) else { close_code = 1007; reason = "Invalid message"; break; };
                let mut rooms = server.0.rooms.lock().unwrap();
                let Some(room) = rooms.get_mut(&code) else { break; };
                match message {
                    ClientMessage::Move { map, x, y, moving, facing, indoors: _, fishing } if x.is_finite() && y.is_finite() => {
                        let Some(layout) = server.0.world.maps.get(&map) else { close_code = 1007; reason = "Unknown map"; break; };
                        let indoors = server.0.world.content.manifest.maps.iter().find(|m| m.id == map).unwrap().indoors;
                        let Some(peer) = room.peers.get_mut(&participant.id) else { break; };
                        let p = &mut peer.player;
                        p.map = map; p.x = x.clamp(12.0, layout.width as f32 * 16.0 - 12.0); p.y = y.clamp(-64.0, layout.origin_y() + 96.0); p.moving = moving; p.facing = facing; p.indoors = indoors; p.fishing = fishing && layout.interaction(p.x, p.y).is_some_and(|o| o.kind == "fishing");
                        let message = ServerMessage::Moved { player: p.clone() };
                        broadcast(room, message, Some(participant.id));
                    }
                    ClientMessage::Chat { text } => {
                        let text = clean(&text, 80);
                        if !text.is_empty() && last_chat.elapsed() >= Duration::from_millis(300) {
                            last_chat = Instant::now(); broadcast(room, ServerMessage::Chat { id: participant.id, text }, None);
                        }
                    }
                    _ => { close_code = 1007; reason = "Invalid activity or repeated map acknowledgement"; break; }
                }
            }
        }
    }
    drop(participant);
    close(&mut socket, close_code, reason).await;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stalled_movement_keeps_the_latest_position_and_preserves_reliable_events() {
        let player = Player {
            id: 7,
            name: "Walker".into(),
            map: "yapshire:town".into(),
            x: 0.0,
            y: 0.0,
            moving: true,
            facing: false,
            indoors: false,
            fishing: false,
        };
        let (send, mut receive) = mpsc::channel(4);
        let (stop, stopped) = watch::channel(false);
        let movements = Arc::new(Mutex::new(HashMap::new()));
        let room = Room {
            name: "Test".into(),
            persistent: false,
            peers: HashMap::from([(
                42,
                Peer {
                    player: Player {
                        id: 42,
                        ..player.clone()
                    },
                    send,
                    movements: movements.clone(),
                    stop,
                },
            )]),
        };
        broadcast(
            &room,
            ServerMessage::Joined {
                player: player.clone(),
            },
            None,
        );
        // A receiver stalls while many updates arrive for the same player.
        for x in 0..1000 {
            broadcast(
                &room,
                ServerMessage::Moved {
                    player: Player {
                        x: x as f32,
                        ..player.clone()
                    },
                },
                None,
            );
        }
        broadcast(
            &room,
            ServerMessage::Chat {
                id: 7,
                text: "Hello".into(),
            },
            None,
        );
        broadcast(&room, ServerMessage::Left { id: 7 }, None);
        assert!(
            !*stopped.borrow(),
            "obsolete movement must not evict a player"
        );
        assert!(matches!(
            receive.try_recv().unwrap(),
            Outgoing::Message(ServerMessage::Joined { .. })
        ));
        assert!(matches!(receive.try_recv().unwrap(), Outgoing::Movement(7)));
        assert_eq!(movements.lock().unwrap().remove(&7).unwrap().x, 999.0);
        assert!(
            matches!(receive.try_recv().unwrap(), Outgoing::Message(ServerMessage::Chat { text, .. }) if text == "Hello")
        );
        assert!(matches!(
            receive.try_recv().unwrap(),
            Outgoing::Message(ServerMessage::Left { id: 7 })
        ));
        assert!(receive.try_recv().is_err());
    }
}
