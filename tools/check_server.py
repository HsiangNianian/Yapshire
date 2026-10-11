"""Exercise a real server with two password-aware WebSocket clients (stdlib only)."""
import argparse
import base64
import hashlib
import json
import os
import socket
import struct
import urllib.error
import urllib.request


def authorization(password):
    return "Bearer " + hashlib.sha256(password.encode()).hexdigest() if password else ""


def http(address, path, password=""):
    request = urllib.request.Request("http://" + address + path, headers={"Authorization": authorization(password)})
    # Local smoke checks must never use a system proxy.
    with urllib.request.build_opener(urllib.request.ProxyHandler({})).open(request, timeout=4) as response:
        return json.load(response)


class Client:
    def __init__(self, address, path, password=""):
        host, port = address.rsplit(":", 1)
        self.socket = socket.create_connection((host, int(port)), timeout=5)
        key = base64.b64encode(os.urandom(16)).decode()
        request = f"GET {path} HTTP/1.1\r\nHost: {address}\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Key: {key}\r\nSec-WebSocket-Version: 13\r\nAuthorization: {authorization(password)}\r\n\r\n"
        self.socket.sendall(request.encode())
        headers = bytearray()
        while not headers.endswith(b"\r\n\r\n"):
            headers.extend(self.exact(1))
            assert len(headers) < 16384, "Oversized handshake"
        assert headers.startswith(b"HTTP/1.1 101"), headers.decode()
        accepted = base64.b64encode(hashlib.sha1((key + "258EAFA5-E914-47DA-95CA-C5AB0DC85B11").encode()).digest())
        assert accepted in headers, "Invalid WebSocket handshake"

    def exact(self, count):
        data = bytearray()
        while len(data) < count:
            chunk = self.socket.recv(count - len(data))
            assert chunk, "Socket closed early"
            data.extend(chunk)
        return data

    def receive(self):
        first, second = self.exact(2)
        assert first == 0x81 and not second & 0x80, "Expected an unfragmented text message"
        size = second & 127
        if size == 126:
            size = struct.unpack("!H", self.exact(2))[0]
        elif size == 127:
            size = struct.unpack("!Q", self.exact(8))[0]
        assert size <= 512 * 1024, "Oversized server message"
        return json.loads(self.exact(size))

    def send(self, value):
        raw = json.dumps(value).encode()
        mask = os.urandom(4)
        header = bytes([0x81, 0x80 | len(raw)]) if len(raw) < 126 else b"\x81\xfe" + struct.pack("!H", len(raw))
        self.socket.sendall(header + mask + bytes(b ^ mask[i % 4] for i, b in enumerate(raw)))

    def close(self):
        self.socket.close()


def check(address, password="", expected_world=None):
    health = http(address, "/health")
    assert health["ok"] and health["protocol"] == 3
    if password:
        try:
            http(address, "/rooms")
            raise AssertionError("Password required for the lobby")
        except urllib.error.HTTPError as error:
            assert error.code == 401
    rooms = http(address, "/rooms", password)["rooms"]
    room = next(item for item in rooms if item["code"] == "MAIN0001")
    assert room["players"] == 0
    clients = []
    try:
        worlds, ids = [], []
        for name in ["Alice", "Bob"]:
            client = Client(address, f"/room/MAIN0001?protocol=3&name={name}", password)
            clients.append(client)
            payload = client.receive()
            assert payload["type"] == "world"
            world = payload["world"]
            assert world["revision"] == health["world"]
            if expected_world:
                for name, map_id in [("town", "yapshire:town"), ("shop", "yapshire:tackle_shop")]:
                    expected = expected_world[name]
                    actual = world["maps"][map_id]
                    assert (actual["width"], actual["height"]) == (expected["width"], expected["height"])
                    assert actual["tilesets"] == expected["tilesets"]
                    assert len(actual["layers"]) == len(expected["layers"])
                    for actual_layer, layer in zip(actual["layers"], expected["layers"]):
                        for key, value in layer.items():
                            assert actual_layer.get(key) == value, (name, key)
            worlds.append(world)
            client.send({"type": "world_ready", "revision": world["revision"]})
            welcome = client.receive()
            assert welcome["type"] == "welcome" and len(welcome["players"]) == len(clients)
            ids.append(welcome["you"])
        assert worlds[0] == worlds[1] and ids[0] != ids[1]
        assert clients[0].receive()["player"]["id"] == ids[1]
        clients[0].send({"type": "chat", "id": ids[1], "text": "你好，自建小镇！"})
        for client in clients:
            message = client.receive()
            assert message == {"type": "chat", "id": ids[0], "text": "你好，自建小镇！"}
        clients[0].send({"type": "move", "map": "yapshire:tackle_shop", "x": 400, "y": 20, "moving": True, "facing": False, "indoors": True, "fishing": False})
        moved = clients[1].receive()
        assert moved["type"] == "moved" and moved["player"]["id"] == ids[0] and moved["player"]["x"] == 400 and moved["player"]["indoors"]
        assert http(address, "/rooms", password)["rooms"][0]["players"] == 2
        clients[0].close()
        assert clients[1].receive() == {"type": "left", "id": ids[0]}
    finally:
        for client in clients:
            client.close()
    print(f"PASS: server {health['version']}, shared map {health['world']}, two players, chat, movement, disconnect")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("address", help="Local server host:port")
    args = parser.parse_args()
    check(args.address, os.environ.get("YAPSHIRE_SERVER_PASSWORD", ""))
