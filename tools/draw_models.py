"""Author structural pixel components, then assemble models without cutting up a painting."""
import random
from PIL import Image, ImageDraw, ImageFont


def build(define, models, font_path):
    font = ImageFont.truetype(str(font_path), 12)

    def surface(name, size, draw):
        image = Image.new("RGBA", size)
        draw(ImageDraw.Draw(image))
        assert image.getbbox(), name
        define(name, image)

    def part(name, x, y, flip=False):
        source = models["yapshire:" + name]["parts"][0]
        return dict(texture=source["texture"], size=source["size"], offset=[x, y], flip_x=flip)

    def model(name, size, parts, anchor=None):
        assert 0 < len(parts) <= 128, name
        assert all(0 <= p["offset"][i] and p["offset"][i] + p["size"][i] <= size[i]
                   for p in parts for i in (0, 1)), name
        define(name, parts=parts, size=list(size), anchor=anchor or [size[0] // 2, size[1]])

    def wood(d, color, light, dark):
        d.rectangle((0, 0, 15, 15), fill=color)
        for y in range(0, 16, 4):
            d.line((0, y, 15, y), fill=dark)
            d.line((0, y + 1, 15, y + 1), fill=light)
        d.line((3, 3, 8, 3), fill=dark)
        d.line((10, 11, 14, 11), fill=light)

    surface("materials/timber", (16, 16), lambda d: wood(d, "#927657", "#b19266", "#615640"))
    surface("materials/painted_wood", (16, 16), lambda d: wood(d, "#6f896a", "#859976", "#54785e"))

    def roof(d, slope=False):
        d.polygon([(31, 0), (0, 47), (31, 47)] if slope else [(0, 0), (15, 0), (15, 15), (0, 15)], fill="#415752")
        height, width = (48, 32) if slope else (16, 16)
        for y in range(height):
            left = max(0, 31 - int(y * 31 / 47)) if slope else 0
            if y % 8 == 0:
                d.line((left, y, width - 1, y), fill="#728074")
            if y % 8 in (2, 3, 4, 5):
                for x in range((y // 8 % 2) * 8, width, 16):
                    if x >= left:
                        d.point((x, y), fill="#293e3d")
        if slope:
            d.line((31, 0, 0, 47), fill="#64766a")

    surface("materials/shingles", (16, 16), roof)
    surface("building/roof_slope", (32, 48), lambda d: roof(d, True))
    surface("building/eave", (16, 5), lambda d: (
        d.rectangle((0, 0, 15, 4), fill="#4d6255"), d.line((0, 0, 15, 0), fill="#85917a")))
    surface("building/chimney", (16, 24), lambda d: (
        d.rectangle((1, 0, 14, 23), fill="#98705b"),
        [d.line((1, y, 14, y), fill="#6a5549") for y in (7, 15, 23)],
        d.rectangle((0, 0, 15, 3), fill="#cfaa81")))

    def window(d):
        d.rectangle((2, 0, 36, 33), fill="#70745b")
        d.rectangle((4, 2, 34, 29), fill="#e6bd80")
        d.rectangle((6, 3, 32, 27), fill="#f4d595")
        d.line((9, 4, 9, 26), fill="#ffe5b4", width=2)
        d.line((18, 3, 18, 28), fill="#9f9069", width=2)
        d.line((4, 16, 34, 16), fill="#9f9069", width=2)
        d.rectangle((0, 32, 39, 36), fill="#5b7760")

    surface("building/window", (40, 37), window)

    def door(d):
        d.rectangle((0, 0, 37, 50), fill="#556d59")
        d.rectangle((4, 3, 33, 48), fill="#94a082")
        d.rectangle((8, 6, 29, 25), fill="#ebcf95")
        d.line((10, 8, 10, 23), fill="#fae7b8", width=2)
        d.rectangle((28, 35, 30, 37), fill="#eed297")
        d.line((0, 51, 37, 51), fill="#e1cda0")

    surface("building/door", (38, 52), door)
    for name, color in [("green", "#4b6455"), ("cream", "#a9a087")]:
        surface("building/awning_" + name, (16, 14), lambda d, c=color: (
            d.rectangle((0, 0, 15, 11), fill=c), d.rectangle((0, 12, 15, 13), fill="#4b695b")))

    def board(name, label, width, height=22):
        surface(name, (width, height), lambda d: (
            d.rectangle((0, 0, width - 1, height - 1), fill="#3f6358"),
            d.line((3, 2, width - 4, 2), fill="#849575"),
            d.text((width // 2, height // 2 + 1), label, font=font, fill="#f4dfac", anchor="mm")))

    for name, width, label in [("shop", 176, "TIDE & TACKLE"), ("street/coffee", 160, "COFFEE"),
                                ("street/flowers", 144, "FLOWERS")]:
        board("signs/" + name.split("/")[-1], label, width - 16)
        parts = [part("materials/timber", x, y) for y in range(48, 144, 16) for x in range(8, width - 8, 16)]
        parts += [part("materials/shingles", x, y) for y in range(0, 48, 16) for x in range(32, width - 32, 16)]
        parts += [part("building/roof_slope", 0, 0), part("building/roof_slope", width - 32, 0, True)]
        parts += [part("building/eave", x, 44) for x in range(0, width, 16)]
        parts += [part("building/window", 18, 95), part("building/window", width - 58, 95),
                  part("building/door", width // 2 - 19, 92), part("signs/" + name.split("/")[-1], 8, 55)]
        parts += [part("building/awning_" + ("cream" if i % 2 else "green"), x, 79)
                  for i, x in enumerate(range(8, width - 8, 16))]
        if name.startswith("street/"):
            for p in parts:
                p["offset"][1] += 16
            parts.insert(0, part("building/chimney", width - 36, 0))
            model(name, (width, 160), parts)
        else:
            model(name, (width, 144), parts)

    def trunk(d, birch=False):
        d.rectangle((1, 0, 6, 23), fill="#3a3730")
        d.rectangle((2, 0, 4, 23), fill="#bdb69a" if birch else "#72664a")
        for y in [4, 13, 21]:
            d.line((2, y, 3 if birch else 5, y), fill="#5e6351" if birch else "#4c4939")

    for name, birch in [("pine", False), ("aspen", True)]:
        surface("trees/" + name + "_bark", (8, 24), lambda d, b=birch: trunk(d, b))
    surface("trees/roots", (24, 12), lambda d: (
        d.polygon([(10, 0), (14, 0), (16, 7), (23, 11), (15, 10), (11, 8), (0, 11), (7, 7)], fill="#4c503c"),
        d.line((12, 1, 13, 7, 19, 10), fill="#807153")))
    surface("trees/branch_fork", (32, 28), lambda d: (
        d.line((0, 26, 13, 18, 30, 3), fill="#655e48", width=3),
        d.line((13, 18, 15, 4, 18, 0), fill="#655e48", width=2),
        d.line((2, 25, 14, 17, 29, 3), fill="#9c9570"),
        d.line((21, 12, 30, 13), fill="#9c9570")))

    def needles(d):
        d.polygon([(0, 0), (7, 6), (10, 2), (15, 10), (19, 7), (26, 16), (31, 16),
                   (29, 21), (24, 19), (21, 23), (15, 19), (11, 22), (7, 16), (0, 14)], fill="#2e443d")
        rng = random.Random(21)
        for x in range(2, 29, 3):
            y = int(x * .5) + rng.randrange(1, 5)
            d.line((x, y - 3, x + 2, y + 2), fill="#425849")
            d.line((x, y, x + 2, y), fill="#667754")
        d.line((0, 11, 25, 19), fill="#3b4c3d")

    surface("trees/pine_bough", (32, 24), needles)
    surface("trees/pine_tip", (16, 28), lambda d: (
        d.line((8, 0, 7, 26), fill="#72664a"),
        d.polygon([(8, 1), (5, 11), (7, 10), (1, 21), (6, 19), (0, 27), (15, 27),
                   (11, 18), (13, 20), (9, 9), (10, 11)], fill="#354b40"),
        d.line((8, 2, 7, 13), fill="#82916c")))
    parts = [part("trees/pine_bark", 44, y) for y in range(12, 180, 24)]
    parts += [part("trees/branch_fork", 47, y) for y in (60, 87, 114)]
    parts += [part("trees/branch_fork", 16, y + 7, True) for y in (60, 87, 114)]
    for i, y in enumerate(range(22, 147, 12)):
        spread = min(i, 7)
        parts += [part("trees/pine_bough", 45 + spread, y),
                  part("trees/pine_bough", 19 - spread, y + 5, True)]
    parts += [part("trees/pine_tip", 40, 0), part("trees/roots", 36, 180)]
    model("trees/pine", (96, 192), parts)

    for variant in range(3):
        def leaves(d, variant=variant):
            rng = random.Random(77 + variant)
            for _ in range(40):
                x, y = rng.randrange(3, 26), rng.randrange(3, 26)
                d.polygon([(x, y), (x + 3, y - 2), (x + 6, y), (x + 4, y + 4), (x - 2, y + 2)],
                          fill=rng.choice(["#78794c", "#9b8b4e", "#bca363", "#d2b576"]))
                d.line((x, y, x + 3, y), fill="#c6ab69")
        surface(f"trees/aspen_leaves_{variant}", (32, 32), leaves)
    parts = [part("trees/aspen_bark", 44, y) for y in range(12, 180, 24)]
    for i, y in enumerate((30, 54, 78, 102, 126)):
        parts += [part("trees/branch_fork", 47, y), part("trees/branch_fork", 16, y + 9, True)]
    for i, (x, y) in enumerate([(32, 0), (49, 15), (17, 25), (33, 36), (60, 42), (4, 57),
                                (29, 64), (58, 77), (14, 87), (39, 102), (24, 122)]):
        parts.append(part(f"trees/aspen_leaves_{i % 3}", x, y, i % 2 == 0))
    parts.append(part("trees/roots", 36, 180))
    model("trees/aspen", (96, 192), parts)

    def frond(d):
        d.line((8, 23, 5, 0), fill="#637b55")
        for y in range(4, 22, 3):
            d.line((int(5 + y / 8), y + 1, 2, y - 2), fill="#82915f")
            d.line((int(5 + y / 8), y + 1, 12, y - 1), fill="#495f44")
    surface("plants/frond", (16, 24), frond)
    model("plants/fern", (48, 32), [part("plants/frond", x, y, x % 8 == 0)
          for x, y in [(0, 8), (8, 3), (16, 0), (24, 5), (32, 8)]])
    surface("street/fence_post", (4, 24), lambda d: (
        d.rectangle((0, 0, 3, 23), fill="#5c6250"), d.line((0, 1, 0, 22), fill="#a59772")))
    surface("street/fence_rail", (24, 3), lambda d: (
        d.rectangle((0, 0, 23, 2), fill="#696d56"), d.line((0, 0, 23, 0), fill="#9b9473")))
    model("street/fence", (24, 24), [part("street/fence_rail", 0, 8), part("street/fence_post", 0, 0)], [0, 24])
    surface("street/lamp_head", (18, 18), lambda d: (
        d.rectangle((4, 5, 13, 17), fill="#465154"), d.rectangle((6, 7, 11, 14), fill="#ffdfa1"),
        d.rectangle((0, 3, 17, 5), fill="#596556"), d.rectangle((5, 0, 12, 2), fill="#596556")))
    surface("street/lamp_pole", (4, 48), lambda d: d.rectangle((0, 0, 3, 47), fill="#455557"))
    surface("street/lamp_foot", (12, 4), lambda d: d.rectangle((0, 0, 11, 3), fill="#485857"))
    model("street/lamp", (18, 68), [part("street/lamp_pole", 7, 17), part("street/lamp_foot", 3, 64), part("street/lamp_head", 0, 0)])
    surface("street/bench_leg", (4, 26), lambda d: d.rectangle((0, 0, 3, 25), fill="#536153"))
    surface("street/bench_plank", (40, 3), lambda d: (
        d.rectangle((0, 0, 39, 2), fill="#92705a"), d.line((0, 0, 39, 0), fill="#c49b70")))
    model("street/bench", (40, 26), [part("street/bench_leg", x, 0) for x in (4, 32)] +
          [part("street/bench_plank", 0, y) for y in (1, 6, 15)], [0, 26])
    surface("street/cup", (6, 6), lambda d: (
        d.rectangle((0, 0, 3, 4), fill="#fff0c8"), d.rectangle((4, 1, 5, 3), outline="#e1cc9e")))
    model("street/cafe_table", (40, 32), [part("street/bench_leg", 18, 6),
          part("street/bench_plank", 0, 6), part("street/cup", 13, 0)])
    surface("street/chalkboard", (24, 32), lambda d: (
        d.rectangle((0, 0, 23, 31), fill="#9b7b59"), d.rectangle((3, 3, 20, 26), fill="#3b5953"),
        [d.line((7, y, 16, y), fill="#ded7a8") for y in (8, 14, 20)]))
    # A sign is a complete surface; only its placement anchor differs from a material's.
    models["yapshire:street/chalkboard"]["anchor"] = [12, 32]
    surface("street/pennant", (8, 12), lambda d: d.polygon([(0, 0), (7, 1), (3, 11)], fill="#cf8f70"))
    surface("street/pennant_rope", (128, 32), lambda d: d.line((0, 0, 78, 22, 127, 0), fill="#56695a"))
    model("street/pennants", (128, 34), [part("street/pennant_rope", 0, 0)] +
          [part("street/pennant", x, round(x * 22 / 78) if x < 78 else round((127 - x) * 22 / 49))
           for x in range(5, 120, 13)], [0, 0])
    surface("street/terracotta", (16, 16), lambda d: (
        d.polygon([(2, 3), (13, 3), (11, 15), (4, 15)], fill="#ba8864"),
        d.rectangle((0, 0, 15, 3), fill="#d3a178")))
    model("street/pot", (16, 34), [part("plants/frond", 0, 0), part("street/terracotta", 0, 18)])
    board("signs/pier", "PIER >", 80)
    board("signs/tackle", "TACKLE >", 80)
    for name, label in [("sign", "pier"), ("tackle_sign", "tackle")]:
        model(name, (80, 80), [part("street/fence_post", 38, y) for y in (8, 32, 56)] + [part("signs/" + label, 0, 0)])

    def barrel(d):
        d.rectangle((2, 2, 21, 21), fill="#6c6552")
        d.rectangle((4, 0, 19, 23), fill="#a98b60")
        for x in [6, 11, 16]:
            d.line((x, 2, x, 21), fill="#806f51")
        for y in [4, 16]:
            d.rectangle((1, y, 22, y + 2), fill="#536d60")
        d.ellipse((3, 0, 20, 5), fill="#3b4033", outline="#ab936b")
    surface("barrel/staves", (24, 24), barrel)
    surface("barrel/lid", (24, 8), lambda d: (
        d.ellipse((3, 1, 20, 6), fill="#c7ac78", outline="#7e7556"),
        d.line((6, 3, 17, 3), fill="#ab936b")))
    model("barrel", (32, 32), [part("barrel/staves", 4, 8), part("barrel/lid", 4, 4)])
    model("barrel_open", (32, 40), [part("barrel/staves", 4, 16), part("barrel/lid", 4, 0)])

    surface("harbor/piling", (8, 16), lambda d: (
        d.rectangle((0, 0, 7, 15), fill="#756450"), d.line((1, 0, 1, 15), fill="#ac9069", width=2)))
    surface("harbor/piling_cap", (12, 5), lambda d: d.rectangle((0, 0, 11, 4), fill="#cbbb8e"))
    surface("harbor/rope_wrap", (12, 8), lambda d: [d.line((1, y, 10, y), fill="#c8b58b") for y in (0, 2, 4, 6)])
    model("post", (16, 80), [part("harbor/piling", 4, y) for y in range(0, 80, 16)] +
          [part("harbor/piling_cap", 2, 0), part("harbor/rope_wrap", 2, 18)])
    board("signs/exit", "EXIT", 64, 20)
    model("door", (64, 80), [part("building/door", 13, 28), part("signs/exit", 0, 0)])
    surface("shop/rod", (12, 76), lambda d: (
        d.line((2, 0, 4, 70), fill="#dbbd81", width=2),
        d.rectangle((2, 57, 5, 75), fill="#8b7052"),
        d.ellipse((4, 53, 10, 59), fill="#b3ae81", outline="#56775c")))
    surface("shop/shelf_board", (16, 8), lambda d: (
        d.rectangle((0, 0, 15, 5), fill="#846f50"), d.line((0, 0, 15, 0), fill="#c1aa7a")))
    parts = [part("materials/painted_wood", x, y) for y in range(16, 96, 16) for x in range(0, 64, 16)]
    parts += [part("shop/rod", x, 16) for x in (6, 19, 32, 45)]
    parts += [part("shop/shelf_board", x, y) for y in (35, 85) for x in range(0, 64, 16)]
    model("rack", (64, 96), parts)
    for variant, color in enumerate(["#c9b884", "#789b8a"]):
        surface(f"shop/bait_jar_{variant}", (14, 24), lambda d, c=color: (
            d.rectangle((2, 5, 11, 23), fill=c), d.rectangle((0, 3, 13, 6), fill="#496e58"),
            d.rectangle((4, 12, 9, 18), fill="#ead8ab"), d.line((4, 8, 4, 10), fill="#e6d3a0")))
    model("shelf", (112, 32), [part("shop/shelf_board", x, 24) for x in range(0, 112, 16)] +
          [part(f"shop/bait_jar_{i % 2}", 3 + i * 17, 0) for i in range(6)])
    surface("shop/register", (24, 12), lambda d: (
        d.rectangle((0, 0, 23, 11), fill="#3e6356"), d.rectangle((4, 2, 19, 6), fill="#ced0a4")))
    board("signs/counter", "TIDE & TACKLE", 112, 14)
    model("counter", (192, 32), [part("materials/painted_wood", x, 16) for x in range(0, 192, 16)] +
          [part("shop/shelf_board", x, 11) for x in range(0, 192, 16)] +
          [part("shop/register", 116, 0), part("signs/counter", 25, 18)])
    def poster(d):
        d.rectangle((0, 0, 63, 63), fill="#71836a")
        d.rectangle((3, 3, 60, 60), fill="#dfd3a6")
        d.text((32, 13), "FRESH", font=font, fill="#4f7460", anchor="mm")
        d.ellipse((12, 24, 44, 40), fill="#70998a", outline="#4c7969")
        d.polygon([(39, 31), (52, 23), (51, 43)], fill="#70998a")
        d.line((18, 27, 34, 27), fill="#b4c5a4")
        d.point((17, 31), fill="#e9ddb1")
        d.text((32, 51), "FISH", font=font, fill="#4f7460", anchor="mm")
    surface("shop/fish_poster", (64, 64), poster)
    model("poster", (64, 64), [part("shop/fish_poster", 0, 0)])

    surface("lighthouse/masonry", (32, 24), lambda d: (
        d.rectangle((0, 0, 31, 23), fill="#b5bca9"), d.rectangle((24, 0, 31, 23), fill="#87988d"),
        d.line((0, 23, 31, 23), fill="#9ca491"), d.rectangle((13, 6, 18, 16), fill="#4b6d70")))
    surface("lighthouse/band", (32, 12), lambda d: d.rectangle((0, 0, 31, 11), fill="#8e7264"))
    surface("lighthouse/lantern", (40, 24), lambda d: (
        d.rectangle((4, 0, 35, 22), fill="#496963"), d.rectangle((8, 4, 31, 18), fill="#ded2a3"),
        d.line((19, 3, 19, 20), fill="#647969", width=2), d.rectangle((0, 21, 39, 23), fill="#778d7c")))
    surface("lighthouse/roof", (48, 16), lambda d: (
        d.polygon([(0, 15), (24, 0), (47, 15)], fill="#425f5b"), d.line((2, 15, 45, 15), fill="#9eaa89")))
    surface("lighthouse/rocks", (64, 24), lambda d: (
        d.polygon([(0, 23), (9, 13), (18, 15), (24, 2), (40, 0), (47, 11), (57, 15), (63, 23)], fill="#576e67"),
        d.line((9, 13, 18, 15, 24, 2, 40, 0), fill="#8e9a84")))
    model("lighthouse", (64, 160), [part("lighthouse/masonry", 16, y) for y in range(32, 152, 24)] +
          [part("lighthouse/band", 16, 67), part("lighthouse/band", 16, 115),
           part("lighthouse/lantern", 12, 14), part("lighthouse/roof", 8, 0), part("lighthouse/rocks", 0, 136)])
