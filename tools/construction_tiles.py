"""Pack foreground artwork into independently editable 16px cells.

Source sheets/prompts: art_sources/. Resample once to the game's pixel density;
render with nearest sampling. Recipes reference cells, never whole-object sprites.
Versioned woodland recipe IDs and dimensions are part of world-save compatibility.
"""
from pathlib import Path
import random
from PIL import Image, ImageDraw, ImageFont, ImageOps

SOURCE = Path(__file__).with_name("art_sources")


def cut(sheet, box, size, *, fit=False):
    im = sheet.crop(box)
    if fit:
        # Ignore invisible RGB fringe only when measuring bounds; preserve alpha.
        bounds = im.getchannel("A").point(lambda a: 255 if a > 8 else 0).getbbox()
        assert bounds, box
        im = ImageOps.contain(im.crop(bounds), size, Image.Resampling.LANCZOS)
        out = Image.new("RGBA", size)
        out.paste(im, ((size[0] - im.width) // 2, size[1] - im.height))
        return out
    return im.resize(size, Image.Resampling.LANCZOS)


def build(groups, prefabs):
    metadata, terrains, images = {}, {}, {}
    architecture = Image.open(SOURCE / "architecture.png").convert("RGBA")
    woodland = Image.open(SOURCE / "woodland.png").convert("RGBA")
    props = Image.open(SOURCE / "props.png").convert("RGBA")
    harbor = Image.open(SOURCE / "harbor.png").convert("RGBA")

    def tile(group, name, label, im, collision="none", layer="decor"):
        assert im.size == (16, 16), name
        key = "yapshire:" + name
        assert key not in images, key
        groups.setdefault(group, []).append((key, im, collision, None))
        metadata[key] = {"label": label, "build_layer": layer}
        images[key] = im
        return key

    def recipe(name, rows):
        prefabs["yapshire:" + name] = dict(width=len(rows[0]), height=len(rows),
            anchor=[len(rows[0]) * 8, len(rows) * 16],
            tiles=[key for row in rows for key in row])
        return rows

    def pieces(group, name, label, im, collision="none", layer="decor", trunk_rows=0):
        rows = []
        for y in range(0, im.height, 16):
            row = []
            for x in range(0, im.width, 16):
                cell = im.crop((x, y, x + 16, y + 16))
                if cell.getchannel("A").getextrema()[1] <= 8:
                    row.append("")
                    continue
                depth = "wall" if y >= im.height - trunk_rows * 16 else layer
                row.append(tile(group, f"{name}/{x // 16}_{y // 16}", label,
                                cell, collision, depth))
            rows.append(row)
        return recipe(name, rows)

    # Re-skin the original harbor cells without changing their stable IDs,
    # collision rules or animation indices. Old edited layouts keep their shape.
    barrel = cut(harbor, (120, 138, 410, 510), (32, 32), fit=True)
    piling = cut(harbor, (708, 24, 830, 520), (16, 80))
    masonry = cut(harbor, (1048, 548, 1484, 955), (32, 32))
    dock = cut(harbor, (564, 549, 971, 953), (32, 16))
    for group in ("objects/harbor", "terrain/ground"):
        for i, (key, im, collision, old) in enumerate(groups[group]):
            name, _, coord = key.removeprefix("yapshire:").partition('/')
            if name in ("barrel", "post", "stone", "deck"):
                x, y = map(int, coord.split('_'))
                source = {"barrel": barrel, "post": piling, "stone": masonry, "deck": dock}[name]
                if name == "stone":
                    x, y = x % 2, x // 2
                im = source.crop((x * 16, y * 16, x * 16 + 16, y * 16 + 16))
                groups[group][i] = key, im, collision, old

    # Ground stays quiet beneath the detailed vegetation.
    for name, label, color, colors, seed in [
        ("earth", "Earth", "#665b45", ["#847354", "#494c3e", "#706343"], 6),
        ("rock", "Rock", "#596466", ["#71807c", "#424e51", "#899188"], 17),
        ("sand", "Sand", "#b4a17a", ["#d0bf8e", "#908b6a", "#c2ad7c"], 40),
        ("grass", "Grassy earth", "#665b45", ["#847354", "#494c3e", "#706343"], 6),
    ]:
        im = Image.new("RGBA", (16, 16), color)
        d, rng = ImageDraw.Draw(im), random.Random(seed)
        for _ in range(26):
            x, y = rng.randrange(16), rng.randrange(16)
            d.line((x, y, min(x + rng.randrange(1, 3), 15), y), fill=rng.choice(colors))
        if name == "grass":
            for x in range(16):
                d.line((x, 0, x, 1 + rng.randrange(3)), fill=rng.choice(["#88905c", "#7a8552", "#a09b69"]))
        tile("terrain/natural", name, label, im, "solid", "solid")

    g = "buildings/modules"
    wood = cut(architecture, (48, 59, 395, 397), (64, 32))
    slate = cut(architecture, (460, 55, 793, 396), (64, 64))
    plaster = cut(architecture, (860, 57, 1200, 394), (64, 64))
    brick = cut(architecture, (588, 916, 704, 1187), (32, 64))
    stone = cut(props, (882, 907, 1182, 1200), (48, 48))
    for name, label, im, layer in [
        ("cedar", "Cedar siding", wood, "wall"),
        ("planks", "Timber block", wood, "solid"),
        ("plaster", "Lime plaster", plaster, "wall"),
        ("brick", "Weathered brick", brick, "solid"),
        ("roof", "Slate shingles", slate, "wall"),
        ("foundation", "Mossy stone foundation", stone, "solid"),
    ]:
        tile(g, name, label, im.crop((0, 0, 16, 16)), "solid", layer)
        pieces(g, name, label, im, "solid", layer)

    # Continuous slate shingles with reusable 45-degree outer edges.
    for name, left in [("roof_left", True), ("roof_right", False)]:
        for y in range(4):
            im = slate.crop((0 if left else 48, y * 16, 16 if left else 64, y * 16 + 16))
            mask = Image.new("L", (16, 16))
            ImageDraw.Draw(mask).polygon([(15, 0), (0, 15), (15, 15)] if left
                else [(0, 0), (15, 15), (0, 15)], fill=255)
            im.putalpha(mask)
            if y == 0:
                tile(g, name, "Slate roof slope", im, "solid", "wall")
            tile(g, f"{name}/{y}", "Slate roof slope", im, "solid", "wall")

    cornice = cut(architecture, (24, 955, 540, 1090), (64, 16), fit=True)
    window = cut(architecture, (85, 456, 382, 818), (32, 48), fit=True)
    door = cut(architecture, (510, 443, 747, 828), (32, 64), fit=True)
    awning = cut(architecture, (805, 539, 1235, 743), (64, 32), fit=True)
    chimney = cut(architecture, (560, 835, 738, 1216), (16, 48), fit=True)
    sign = cut(architecture, (766, 946, 1232, 1161), (64, 32), fit=True)
    for name, label, im in [
        ("sash_window", "Sash window", window), ("cottage_door", "Cottage door", door),
        ("cornice", "Cedar cornice", cornice), ("striped_awning", "Striped awning", awning),
        ("chimney", "Brick chimney", chimney), ("shop_sign", "Shop sign", sign),
    ]:
        pieces(g, name, label, im, "none", "decor")
    named_sign = sign.copy()
    font = ImageFont.truetype(str(SOURCE.parents[1] / "assets/fonts/fusion-pixel.ttf"), 8)
    ImageDraw.Draw(named_sign).text((32, 22), "TIDE & TACKLE", font=font, fill="#ecdbb5", anchor="mm")
    pieces(g, "tackle_name", "Tide & Tackle sign", named_sign)

    # Backed modules can share a layer with wall cells at transparent edges.
    # These are reusable windows/doors, not a flattened facade or building.
    def backed(name, label, im, material):
        canvas = Image.new("RGBA", im.size)
        for y in range(0, im.height, material.height):
            for x in range(0, im.width, material.width):
                canvas.paste(material, (x, y))
        canvas.alpha_composite(im)
        return pieces(g, name, label, canvas, "solid", "wall")

    backed("cedar_window", "Window in cedar wall", window, wood)
    backed("plaster_window", "Window in plaster wall", window, plaster)
    backed("cedar_door", "Door in cedar wall", door, wood)
    backed("plaster_door", "Door in plaster wall", door, plaster)
    # Preserve IDs used by initial construction saves and quick selection.
    tile(g, "window", "Small sash window", cut(architecture, (85, 456, 382, 818), (16, 16), fit=True), "solid", "wall")
    small_door = cut(architecture, (510, 443, 747, 828), (16, 32), fit=True)
    tile(g, "door_top", "Door upper half", small_door.crop((0, 0, 16, 16)), "solid", "wall")
    tile(g, "door_bottom", "Door lower half", small_door.crop((0, 16, 16, 32)), "solid", "wall")
    tile(g, "beam", "Timber beam", cornice.crop((16, 0, 32, 16)), "solid", "wall")
    tile(g, "awning_green", "Sage awning", awning.crop((16, 16, 32, 32)))
    tile(g, "awning_cream", "Linen awning", awning.crop((32, 16, 48, 32)))
    platform = Image.new("RGBA", (16, 16))
    platform.paste(wood.crop((16, 0, 32, 6)), (0, 0))
    tile(g, "platform", "Timber platform", platform, "platform", "solid")

    g = "nature/garden"
    cedar = cut(woodland, (65, 15, 620, 972), (112, 192), fit=True)
    birch = cut(woodland, (690, 15, 1220, 973), (112, 192), fit=True)
    shrub = cut(woodland, (68, 1005, 620, 1240), (64, 32), fit=True)
    autumn = cut(woodland, (684, 985, 1220, 1238), (64, 32), fit=True)
    tile(g, "trunk", "Cedar trunk", cedar.crop((48, 160, 64, 176)), layer="wall")
    tile(g, "birch", "Birch trunk", birch.crop((48, 160, 64, 176)), layer="wall")

    # Nine-slice the foliage source to retain soft edges and the established
    # sixteen neighbour-mask IDs for individually placed leaves.
    for name, label, box in [
        ("leaves", "Cedar leaves", (85, 1010, 610, 1235)),
        ("gold_leaves", "Autumn leaves", (695, 996, 1215, 1232)),
        ("bush", "Shrub leaves", (85, 1010, 610, 1235)),
    ]:
        leaf = cut(woodland, box, (32, 32))
        variants = []
        for mask in range(16):
            im = Image.new("RGBA", (16, 16))
            for dx, dy, horizontal, vertical in [(0, 0, 8, 1), (8, 0, 2, 1), (0, 8, 8, 4), (8, 8, 2, 4)]:
                sx = (8 if dx == 0 else 16) if mask & horizontal else (0 if dx == 0 else 24)
                sy = (8 if dy == 0 else 16) if mask & vertical else (0 if dy == 0 else 24)
                im.paste(leaf.crop((sx, sy, sx + 8, sy + 8)), (dx, dy))
            key = tile(g, f"{name}/{mask}", label, im)
            metadata[key]["terrain"] = "yapshire:" + name
            variants.append(key)
        terrains["yapshire:" + name] = variants
    for name, label, box in [
        ("tuft", "Wild grass", (30, 940, 407, 1205)),
        ("flowers", "Wild flowers", (455, 940, 818, 1206)),
    ]:
        tile(g, name, label, cut(props, box, (16, 16), fit=True))

    # Put common trunks/leaves first in the palette, followed by detailed parts.
    for name, label, im, trunks in [
        ("cedar_tree/v2", "Cedar branches", cedar, 2),
        ("birch_tree/v2", "Birch branches", birch, 2),
        ("shrub/v2", "Green shrub", shrub, 0),
        ("autumn_shrub/v2", "Autumn shrub", autumn, 0),
    ]:
        rows = pieces(g, name, label, im, trunk_rows=trunks)
        recipe(name.split('/')[0], rows)

    g = "objects/street"
    for name, label, box, size in [
        ("street_lamp", "Street lantern", (160, 23, 268, 571), (16, 80)),
        ("garden_bench", "Garden bench", (424, 330, 828, 568), (48, 32)),
        ("picket_fence", "Picket fence", (855, 302, 1222, 560), (48, 32)),
        ("flower_planter", "Flower planter", (72, 590, 387, 887), (32, 32)),
        ("standing_board", "Chalkboard", (894, 589, 1167, 882), (16, 32)),
    ]:
        pieces(g, name, label, cut(props, box, size, fit=True))
    for name, label, box in [
        ("fence", "Fence", (866, 315, 1212, 548)),
        ("planter", "Flower pot", (72, 590, 387, 887)),
        ("lamp", "Lantern", (164, 25, 261, 195)),
        ("lamp_post", "Lamp post", (156, 270, 270, 384)),
        ("bench", "Bench seat", (453, 354, 787, 481)),
        ("bench_leg", "Bench leg", (425, 451, 517, 561)),
        ("crate", "Wooden crate", (483, 621, 766, 875)),
        ("chalkboard", "Chalkboard", (894, 589, 1167, 882)),
    ]:
        tile(g, name, label, cut(props, box, (16, 16), fit=True),
             "solid" if name == "crate" else "none", "solid" if name == "crate" else "decor")
    rope = cut(harbor, (39, 663, 500, 851), (64, 16))
    pieces(g, "mooring_rope", "Mooring rope", rope)
    for name, text in [("pier_waymark", "PIER"), ("tackle_waymark", "TACKLE")]:
        board = cut(harbor, (1048, 46, 1498, 516), (80, 80), fit=True)
        font = ImageFont.truetype(str(SOURCE.parents[1] / "assets/fonts/fusion-pixel.ttf"), 12)
        ImageDraw.Draw(board).text((38, 22), text, font=font, fill="#ecdbb5", anchor="mm")
        pieces(g, name, "Harbor waymark", board)

    def stamp(rows, x, y, module):
        for dy, row in enumerate(module):
            for dx, key in enumerate(row):
                if key:
                    rows[y + dy][x + dx] = key

    def module(name):
        p = prefabs["yapshire:" + name]
        return [p["tiles"][y * p["width"]:(y + 1) * p["width"]] for y in range(p["height"])]

    def over(rows, x, y, module, name, label):
        """Overlay at cell level, preserving the wall texture's exact phase."""
        for dy, row in enumerate(module):
            for dx, foreground in enumerate(row):
                if not foreground:
                    continue
                background = rows[y + dy][x + dx]
                if not background:
                    rows[y + dy][x + dx] = foreground
                    continue
                variant = f"{name}/{background.split(':')[1].replace('/', '_')}_{dx}_{dy}"
                key = "yapshire:" + variant
                if key not in images:
                    im = images[background].copy()
                    im.alpha_composite(images[foreground])
                    tile("buildings/modules", variant, label, im, "solid", "wall")
                rows[y + dy][x + dx] = key

    def house(name, material, shop=False):
        rows = [[""] * 12 for _ in range(11)]
        stamp(rows, 9, 0, module("chimney"))
        for y in range(1, 5):
            lo, hi = 4 - y, 7 + y
            for x in range(lo, hi + 1):
                key = "yapshire:" + (f"roof_left/{y - 1}" if x == lo else
                    f"roof_right/{y - 1}" if x == hi else f"roof/{x % 4}_{(y - 1) % 4}")
                if rows[y][x] and (x == lo or x == hi):
                    over(rows, x, y, [[key]], "roof_chimney", "Roof around chimney")
                else:
                    rows[y][x] = key
        for y in range(5, 11):
            for x in range(1, 11):
                rows[y][x] = f"yapshire:{material}/{x % 4}_{y % (2 if material == 'cedar' else 4)}"
        for x in range(0, 12, 4):
            over(rows, x, 4, module("cornice"), "cornice_roof", "Cedar roof eave")
        for x in range(1, 11):
            rows[10][x] = f"yapshire:foundation/{x % 3}_1"
        over(rows, 2, 6, module("sash_window"), material + "_window", "Window in wall")
        over(rows, 8, 6, module("sash_window"), material + "_window", "Window in wall")
        over(rows, 5, 7, module("cottage_door"), material + "_door", "Door in wall")
        if shop:
            over(rows, 4, 5, module("tackle_name"), "shop_sign_backed", "Tackle shop sign")
        recipe(name, rows)

    house("cottage", "cedar")
    house("plaster_cottage", "plaster")
    house("harbor_shop", "cedar", shop=True)
    return metadata, terrains


def town_layers(width, height, recipes):
    """Sparse layouts; each silhouette and all its details remain editable cells."""
    layers = {name: [""] * (width * height) for name in ("Street trees", "Street houses", "Street garden")}

    def stamp(layer, x, bottom, name):
        p = recipes["yapshire:" + name]
        for i, key in enumerate(p["tiles"]):
            cx, cy = x + i % p["width"], bottom - p["height"] + i // p["width"]
            if key and 0 <= cx < width and 0 <= cy < height:
                layers[layer][cy * width + cx] = key

    for x, name in [(0, "cedar_tree"), (22, "birch_tree"), (30, "cedar_tree"), (51, "cedar_tree")]:
        stamp("Street trees", x, 13, name)
    stamp("Street houses", 8, 13, "cottage")
    stamp("Street houses", 37, 13, "plaster_cottage")
    stamp("Street houses", 54, 13, "harbor_shop")
    for x in (0, 3, 21, 24, 27, 30, 33, 50, 53):
        stamp("Street garden", x, 13, "picket_fence")
    for x, name in [(0, "shrub"), (3, "autumn_shrub"), (21, "shrub"),
                    (31, "shrub"), (49, "autumn_shrub"), (54, "shrub")]:
        stamp("Street garden", x, 13, name)
    for x in (7, 20, 35, 50):
        stamp("Street garden", x, 13, "street_lamp")
    stamp("Street garden", 27, 13, "garden_bench")
    for x in (8, 18, 37, 47):
        stamp("Street garden", x, 13, "flower_planter")
    stamp("Street garden", 11, 13, "standing_board")
    stamp("Street garden", 47, 13, "tackle_waymark")
    stamp("Street garden", 71, 13, "pier_waymark")
    for x in (68, 72, 76, 80):
        stamp("Street garden", x, 13, "mooring_rope")
    return layers
