"""Rebuild the official content pack: uv run --with Pillow tools/draw_maps.py.

This overwrites the official pack layouts. Legacy assets/maps fixtures stay read-only.
"""

from pathlib import Path
import random
from PIL import Image, ImageDraw, ImageFont

ASSETS = Path(__file__).resolve().parents[1] / "assets"
TILE, HEIGHT = 16, 17
random.seed(73)
tiles, animations = [], []
font = ImageFont.truetype(str(ASSETS / "fonts/fusion-pixel.ttf"), 12)


def canvas(w=16, h=16, color=(0, 0, 0, 0)):
    im = Image.new("RGBA", (w, h), color)
    return im, ImageDraw.Draw(im)


def tile(im):
    assert im.size == (TILE, TILE)
    tiles.append(im)
    return len(tiles)  # Tiled GIDs are one-based; zero is an empty cell.


def grain(d, box, colors, count):
    x, y, right, bottom = box
    for _ in range(count):
        xx, yy = random.randint(x, right), random.randint(y, bottom)
        d.line((xx, yy, min(xx + random.randrange(1, 4), right), yy), fill=random.choice(colors))


def stamp(im):
    assert im.width % TILE == im.height % TILE == 0
    return [[tile(im.crop((x, y, x + TILE, y + TILE)))
             if im.crop((x, y, x + TILE, y + TILE)).getbbox() else 0
             for x in range(0, im.width, TILE)] for y in range(0, im.height, TILE)]


def put(layer, x, y, pattern):
    for dy, row in enumerate(pattern):
        for dx, gid in enumerate(row):
            if gid:
                assert 0 <= y + dy < HEIGHT and 0 <= x + dx < len(layer[0])
                layer[y + dy][x + dx] = gid


def layer(width):
    return [[0] * width for _ in range(HEIGHT)]


def make_map(width, layers):
    data = {
        "type": "map", "version": "1.10", "orientation": "orthogonal",
        "renderorder": "right-down", "infinite": False,
        "width": width, "height": HEIGHT, "tilewidth": TILE, "tileheight": TILE,
        "nextlayerid": len(layers) + 1, "nextobjectid": 1,
        "tilesets": [{"firstgid": 1, "source": "harbor.tsj"}],
        "layers": [dict(id=i + 1, name=label, type="tilelayer", x=0, y=0,
                        width=width, height=HEIGHT, visible=True, opacity=1,
                        data=[gid for row in cells for gid in row])
                   for i, (label, cells) in enumerate(layers)],
    }
    return data


# Mossy earth and weathered slate keep the walking edge readable without a bright curb.
ground = []
for index in range(4):
    im, d = canvas(color="#706950" if index < 2 else "#4b5044")
    grain(d, (0, 0, 15, 15), ["#5b5b47", "#867c57", "#666c4d"], 17)
    if index < 2:
        d.line((0, 0, 15, 0), fill="#a99a6c")
        d.line((0, 1, 15, 1), fill="#788052")
        for x in range(index, 16, 5):
            d.line((x, 0, x + 1, 2 + x % 3), fill="#92965e")
    else:
        d.line((0, 15, 15, 15), fill="#424b40")
    ground.append(tile(im))

stone = []
for index in range(4):
    im, d = canvas(color="#53646a")
    for y in [0, 8]:
        offset = (y // 8 + index) % 2 * 8
        d.line((0, y, 15, y), fill="#35494d")
        d.line((0, y + 1, 15, y + 1), fill="#829085")
        d.line((offset, y, offset, y + 7), fill="#35494d")
    grain(d, (0, 2, 15, 15), ["#677b7a", "#4b6264", "#8c967f"], 10)
    if index < 2:
        d.line((0, 0, 15, 0), fill="#b7ad8b")
        d.line((0, 2, 15, 2), fill="#7b8270")
    stone.append(tile(im))

deck = []
for index in range(2):
    im, d = canvas()
    d.rectangle((0, 0, 15, 7), fill="#534d3e")
    d.rectangle((0, 0, 15, 1), fill="#b39a6d")
    d.line((0, 3, 15, 3), fill="#8b7956")
    d.line((15, 0, 15, 7), fill="#303a33")
    d.line((2, 5, 12, 5), fill="#796447")
    for x in [2, 12]:
        d.point((x, 1), fill="#77755c")
    deck.append(tile(im))

# Four consecutive frames per water band: slow, sparse glints, not a moving wall.
water = []
for band in range(9):
    variants = []
    for variant in range(3):
        first = len(tiles)
        for frame in range(4):
            im, d = canvas()
            for y in range(16):
                t = (band * 16 + y) / 143
                base = tuple(round(a + (b - a) * t) for a, b in zip((91, 133, 148), (25, 55, 69)))
                # Transparent distance blends the animated water into the painted lake.
                alpha = round(max(0, min(1, (band * 16 + y - 35) / 35)) * 255)
                d.line((0, y, 15, y), fill=(*base, alpha))
            # Some cells remain quiet so there is no obvious checkerboard of waves.
            if variant < 2 and band >= 3:
                start = (variant * 7 + band * 3 + frame) % 12
                y = (band * 5 + variant * 7) % 14 + 1
                d.line((start, y, min(start + 4, 15), y),
                       fill="#b9c5b4" if band < 5 else "#607f84")
                if variant == 0 and band in (4, 6):
                    d.line((2 + frame, 11, 5 + frame, 11), fill="#b7a470")
            tile(im)
        animations.append({"id": first, "animation": [
            {"tileid": first + frame, "duration": 600} for frame in range(4)]})
        variants.append(first + 1)
    water.append(variants)

# Multi-tile stamps retain hand-drawn outlines, while terrain and walls repeat.
shop, d = canvas(176, 144)
d.rectangle((8, 44, 167, 143), fill="#3b443a")
d.rectangle((11, 48, 163, 141), fill="#927657")
d.rectangle((156, 48, 163, 141), fill="#5e5743")
for y in range(49, 140, 5):
    d.line((11, y, 155, y), fill="#615640")
    d.line((12, y + 1, 155, y + 1), fill="#b19266")
d.polygon([(0, 47), (27, 5), (143, 5), (175, 47)], fill="#253c3b")
d.polygon([(3, 41), (29, 3), (143, 3), (172, 41)], fill="#415752")
for y in range(7, 42, 6):
    left, right = int(29 - (y - 3) * .7), int(143 + (y - 3) * .76)
    d.line((left, y, right, y), fill="#728074")
    for x in range(left + 4 + (y % 4), right, 12):
        d.line((x, y + 1, x, y + 4), fill="#49695e")
d.rectangle((2, 44, 173, 48), fill="#4d6255")
d.rectangle((7, 56, 168, 77), fill="#3f6358")
d.line((12, 58, 162, 58), fill="#849575")
d.text((88, 65), "TIDE & TACKLE", font=font, fill="#f4dfac", anchor="mm")
for x in range(8, 168, 10):
    d.polygon([(x, 78), (x + 9, 78), (x + 11, 87), (x - 2, 87)],
              fill="#a9a087" if x % 20 == 8 else "#4b6455")
d.rectangle((6, 87, 169, 89), fill="#4b695b")
for x in [22, 119]:
    d.rectangle((x - 2, 95, x + 32, 128), fill="#70745b")
    d.rectangle((x, 97, x + 30, 124), fill="#e6bd80")
    d.rectangle((x + 2, 98, x + 28, 122), fill="#f4d595")
    d.line((x + 5, 99, x + 5, 121), fill="#ffe5b4", width=2)
    d.line((x + 14, 98, x + 14, 123), fill="#9f9069", width=2)
    d.line((x, 111, x + 30, 111), fill="#9f9069", width=2)
    d.rectangle((x - 4, 127, x + 35, 130), fill="#5b7760")
d.rectangle((68, 92, 105, 142), fill="#556d59")
d.rectangle((72, 95, 101, 140), fill="#94a082")
d.rectangle((76, 98, 97, 117), fill="#ebcf95")
d.line((78, 100, 78, 115), fill="#fae7b8", width=2)
d.rectangle((96, 127, 98, 129), fill="#eed297")
d.rectangle((64, 141, 109, 143), fill="#e1cda0")
shop_stamp = stamp(shop)

barrel, d = canvas(32, 32)
d.rectangle((6, 10, 24, 29), fill="#6c6552")
d.rectangle((8, 8, 22, 30), fill="#a98b60")
for x in [10, 15, 20]:
    d.line((x, 10, x, 28), fill="#806f51")
for y in [12, 24]:
    d.rectangle((5, y, 25, y + 2), fill="#536d60")
d.ellipse((7, 6, 23, 11), fill="#c7ac78", outline="#7e7556")
barrel_stamp = stamp(barrel)

post, d = canvas(16, 80)
d.rectangle((5, 2, 10, 79), fill="#756450")
d.line((6, 4, 6, 79), fill="#ac9069", width=2)
d.rectangle((3, 0, 12, 4), fill="#cbbb8e")
for y in [18, 20, 22]:
    d.line((4, y, 11, y), fill="#c8b58b")
d.rectangle((5, 60, 10, 79), fill="#6b6c50")
d.line((6, 60, 6, 79), fill="#91906a")
post_stamp = stamp(post)

rope, d = canvas()
d.line((0, 3, 8, 5, 15, 3), fill="#a29069")
d.line((0, 2, 8, 4, 15, 2), fill="#d6c39a")
rope_tile = tile(rope)

sign, d = canvas(80, 80)
d.rectangle((38, 17, 41, 79), fill="#7e735a")
d.rectangle((1, 2, 78, 22), fill="#365c53")
d.line((4, 4, 75, 4), fill="#8b9a74")
d.text((40, 13), "PIER >", font=font, fill="#efdcac", anchor="mm")
sign_stamp = stamp(sign)

lighthouse, d = canvas(64, 32)
d.polygon([(0, 31), (16, 27), (50, 27), (63, 31)], fill="#496a70")
d.polygon([(28, 28), (30, 8), (35, 8), (38, 28)], fill="#afbdb3")
d.polygon([(34, 8), (35, 8), (38, 28), (34, 28)], fill="#7f9693")
d.rectangle((29, 18, 36, 21), fill="#8e8980")
d.rectangle((28, 7, 38, 8), fill="#527779")
d.rectangle((30, 3, 35, 6), fill="#d5cbaa")
d.polygon([(27, 2), (33, 0), (39, 2)], fill="#527779")
lighthouse_stamp = stamp(lighthouse)

# The street meets a stone quay, then continues across a timber pier at y = 0.
sea, shore, terrain, buildings, props = [layer(90) for _ in range(5)]
for y in range(8, HEIGHT):
    for x in range(57, 90):
        sea[y][x] = random.choice(water[y - 8])
for y in range(13, HEIGHT):
    for x in range(90):
        if x < 55:
            terrain[y][x] = ground[x % 2 if y == 13 else 2 + (x + y) % 2]
        elif x < 67 - max(y - 14, 0):
            terrain[y][x] = stone[x % 2 if y == 13 else 2 + (x + y) % 2]
        elif y == 13 and x < 85:
            terrain[y][x] = deck[x % 2]
put(buildings, 55, 4, shop_stamp)
put(shore, 85, 10, lighthouse_stamp)
for x in [68, 72, 76, 80, 84]:
    put(shore, x, 11, post_stamp)
for x in range(68, 81):
    buildings[12][x] = rope_tile
for x in [54, 65]:
    put(props, x, 11, barrel_stamp)
put(props, 71, 8, sign_stamp)
sign, d = canvas(80, 80)
d.rectangle((38, 17, 41, 79), fill="#7e735a")
d.rectangle((1, 2, 78, 22), fill="#365c53")
d.line((4, 4, 75, 4), fill="#8b9a74")
d.text((40, 13), "TACKLE >", font=font, fill="#efdcac", anchor="mm")
put(props, 47, 8, stamp(sign))

# Indoor walls and floor are reusable tiles too; furniture is multi-tile artwork.
wall = []
for index in range(3):
    im, d = canvas(color="#594d40")
    d.line((0, 0, 0, 15), fill="#343930")
    d.line((1, 0, 1, 15), fill="#857153")
    grain(d, (3, 0, 14, 15), ["#655742", "#776248"], 6)
    if index == 1:
        d.rectangle((0, 0, 15, 3), fill="#637e65")
        d.line((0, 4, 15, 4), fill="#c8c9a3")
    if index == 2:
        d.rectangle((0, 12, 15, 15), fill="#627e65")
        d.line((0, 11, 15, 11), fill="#d3cba3")
    wall.append(tile(im))
floor = []
for index in range(2):
    im, d = canvas(color="#6d5e46")
    d.line((0, 0, 15, 0), fill="#6e7154")
    d.line((0, 1, 15, 1), fill="#c4ac7d")
    d.line((index * 8, 0, index * 8, 15), fill="#847650")
    grain(d, (1, 3, 14, 14), ["#867351", "#5b533f", "#93805c"], 9)
    floor.append(tile(im))
dark = tile(canvas(color="#2c4946")[0])

door, d = canvas(64, 80)
d.rectangle((12, 22, 51, 79), fill="#4f705e")
d.rectangle((16, 26, 47, 79), fill="#c6b889")
d.rectangle((19, 29, 44, 53), fill="#81a5a0")
d.rectangle((21, 31, 23, 50), fill="#b8d0b6")
d.line((32, 29, 32, 53), fill="#9c916d", width=2)
d.rectangle((41, 64, 43, 66), fill="#7e7b58")
d.rectangle((10, 77, 53, 79), fill="#e0c797")
d.text((32, 10), "EXIT", font=font, fill="#395e50", anchor="mm")
door_stamp = stamp(door)

rack, d = canvas(64, 96)
d.rectangle((3, 8, 60, 95), fill="#67826a", outline="#456956", width=2)
for x in [11, 24, 37, 50]:
    d.line((x, 16, x + 2, 84), fill="#dbbd81", width=2)
    for y in range(19, 67, 11):
        d.point((x + 2, y), fill="#f4dca2")
    d.rectangle((x, 73, x + 3, 87), fill="#8b7052")
    d.ellipse((x + 2, 69, x + 7, 75), fill="#b3ae81", outline="#56775c")
for y in [35, 85]:
    d.rectangle((1, y, 62, y + 3), fill="#ac956b")
rack_stamp = stamp(rack)

poster, d = canvas(64, 64)
d.rectangle((0, 0, 63, 63), fill="#71836a")
d.rectangle((3, 3, 60, 60), fill="#dfd3a6")
d.text((32, 13), "FRESH", font=font, fill="#4f7460", anchor="mm")
d.ellipse((12, 24, 44, 40), fill="#70998a", outline="#4c7969")
d.polygon([(39, 31), (52, 23), (51, 43)], fill="#70998a")
d.line((18, 27, 34, 27), fill="#b4c5a4")
d.point((17, 31), fill="#e9ddb1")
d.text((32, 51), "FISH", font=font, fill="#4f7460", anchor="mm")
poster_stamp = stamp(poster)

shelf, d = canvas(112, 32)
d.rectangle((0, 24, 111, 28), fill="#846f50")
d.line((0, 24, 111, 24), fill="#c1aa7a")
for x in range(6, 109, 17):
    d.rectangle((x, 7, x + 10, 23), fill="#c9b884" if x % 2 else "#789b8a")
    d.rectangle((x - 1, 5, x + 11, 8), fill="#496e58")
    d.rectangle((x + 2, 13, x + 8, 18), fill="#ead8ab")
    d.line((x + 2, 10, x + 2, 12), fill="#e6d3a0")
shelf_stamp = stamp(shelf)

counter, d = canvas(192, 32)
d.rectangle((3, 14, 188, 31), fill="#6f896a")
for x in range(7, 188, 16):
    d.line((x, 14, x, 30), fill="#54785e")
    d.line((x + 1, 14, x + 1, 30), fill="#859976")
d.rectangle((0, 11, 191, 14), fill="#b69e71")
d.line((0, 11, 191, 11), fill="#e4c796", width=2)
d.rectangle((116, 0, 138, 10), fill="#3e6356")
d.rectangle((120, 2, 135, 6), fill="#ced0a4")
d.text((84, 23), "TIDE & TACKLE", font=font, fill="#e9d7a6", anchor="mm")
counter_stamp = stamp(counter)

interior = [layer(30) for _ in range(5)]
for y in range(HEIGHT):
    for x in range(30):
        interior[0][y][x] = dark
        if 1 <= x < 29:
            if 3 <= y < 13:
                interior[1][y][x] = wall[1 if y == 3 else 2 if y == 12 else 0]
            elif y >= 13:
                interior[2][y][x] = floor[(x + y) % 2]
put(interior[3], 2, 8, door_stamp)
put(interior[3], 6, 7, rack_stamp)
put(interior[3], 11, 7, poster_stamp)
put(interior[3], 20, 6, shelf_stamp)
put(interior[3], 20, 8, shelf_stamp)
put(interior[4], 15, 11, counter_stamp)

maps = {
    "town": make_map(90, list(zip(["Water", "Shore and pilings", "Terrain", "Buildings", "Props"],
                                  [sea, shore, terrain, buildings, props]))),
    "tackle-shop": make_map(30, list(zip(["Backdrop", "Walls", "Floor", "Furniture", "Counter"], interior))),
}

for name, width in [("town", 90), ("tackle-shop", 30)]:
    data = maps[name]
    for item in data["layers"]:
        assert len(item["data"]) == width * HEIGHT
        assert all(0 <= gid <= len(tiles) for gid in item["data"])
    # A continuous walking surface matches the current side-scroller physics.
    assert all(data["layers"][2]["data"][13 * width + x] for x in range(1, 85 if width == 90 else width - 1))
print(f"Wrote two Tiled maps and {len(tiles)} tiles ({len(animations)} water animations).")

from build_content import build
build(globals())
