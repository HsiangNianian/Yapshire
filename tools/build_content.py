"""Export the authored pixel art as the official, editable content pack."""

import json
from copy import deepcopy
from pathlib import Path
from PIL import Image, ImageDraw


def build(g):
    root = g["ASSETS"] / "packs/yapshire"
    root.mkdir(parents=True, exist_ok=True)
    groups = {name: [] for name in ("terrain/ground", "terrain/water", "buildings/tackle", "objects/harbor")}
    names, prefabs = {}, {}

    def register(group, name, pattern, collision="none", prefab=False):
        rows = pattern if isinstance(pattern[0], list) else [pattern]
        ids = []
        for y, row in enumerate(rows):
            for x, gid in enumerate(row):
                key = f"yapshire:{name}/{x}_{y}"
                ids.append(key if gid else "")
                if gid:
                    assert gid not in names
                    names[gid] = key
                    groups[group].append((key, g["tiles"][gid - 1], collision, gid))
        if prefab:
            prefabs[f"yapshire:{name}"] = dict(width=len(rows[0]), height=len(rows),
                anchor=[len(rows[0]) * 8, len(rows) * 16], tiles=ids)

    for name in ("ground", "stone", "deck", "floor"):
        register("terrain/ground", name, g[name], "platform" if name == "deck" else "solid")
    register("terrain/water", "water", list(range(11, 119)))
    for name in ("shop", "door", "wall"):
        register("buildings/tackle", name, g[name + "_stamp"] if name != "wall" else g[name], prefab=name != "wall")
    register("buildings/tackle", "indoor_backdrop", [g["dark"]])
    for name in ("barrel", "post", "sign", "lighthouse", "rack", "poster", "shelf", "counter"):
        register("objects/harbor", name, g[name + "_stamp"], prefab=True)
    register("objects/harbor", "rope", [g["rope_tile"]])
    # The second sign is the only unnamed stamp in the original source.
    register("objects/harbor", "tackle_sign", [i for i in range(1, len(g["tiles"]) + 1) if i not in names])
    assert len(names) == len(g["tiles"]) == 359

    # A complete 16-pattern edge terrain, shared by Tiled's Terrain brush and ours.
    terrain_ids = []
    for mask in range(16):
        key = f"yapshire:quay/{mask}"
        terrain_ids.append(key)
        im = g["tiles"][g["stone"][2] - 1].copy()
        d = ImageDraw.Draw(im)
        for bit, line, color in [(1, (0, 0, 15, 0), "#b7ad8b"), (2, (15, 0, 15, 15), "#35494d"),
                                 (4, (0, 15, 15, 15), "#35494d"), (8, (0, 0, 0, 15), "#829085")]:
            if not mask & bit:
                d.line(line, fill=color, width=2)
        groups["terrain/ground"].append((key, im, "solid", None))

    def write(path, value):
        target = root / path
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(json.dumps(value, indent=2, ensure_ascii=False) + "\n")

    def prop(name, value):
        return dict(name=name, type="bool" if isinstance(value, bool) else "string", value=value)

    gids, references, firstgid = {}, [], 1
    palette = {}
    animations = {t["id"] + 1: t["animation"] for t in g["animations"]}
    for group, items in groups.items():
        columns = 16
        atlas = Image.new("RGBA", (columns * 16, ((len(items) + 15) // 16) * 16))
        old_local = {old: i for i, (_, _, _, old) in enumerate(items) if old}
        tiles = []
        for i, (key, im, collision, old) in enumerate(items):
            atlas.paste(im, (i % columns * 16, i // columns * 16))
            gids[key] = firstgid + i
            tile = dict(id=i, properties=[prop("id", key), prop("collision", collision)])
            if key in terrain_ids:
                tile["properties"].append(prop("terrain", "yapshire:quay"))
            if old in animations:
                tile["animation"] = [dict(tileid=old_local[f["tileid"] + 1], duration=f["duration"]) for f in animations[old]]
            tiles.append(tile)
        image = Path(group).name + ".png"
        ts = dict(type="tileset", version="1.10", name=group, tilewidth=16, tileheight=16,
                  tilecount=len(items), columns=columns, margin=0, spacing=0, image=image,
                  imagewidth=atlas.width, imageheight=atlas.height, tiles=tiles)
        if group == "terrain/ground":
            start = len(items) - 16
            ts["wangsets"] = [dict(name="Quay", type="edge", tile=start + 15,
                colors=[dict(name="Stone", color="#7e8171", tile=start + 15, probability=1)],
                wangtiles=[dict(tileid=start + mask, wangid=[int(bool(mask & bit)) if bit else 0
                    for bit in (1, 0, 2, 0, 4, 0, 8, 0)]) for mask in range(16)])]
        write(group + ".tsj", ts)
        atlas.save(root / (group + ".png"))
        palette[group + ".tsj"] = [key for key, _, _, _ in items]
        references.append(dict(firstgid=firstgid, source="../" + group + ".tsj"))
        firstgid += len(items)

    (root / "backgrounds").mkdir(exist_ok=True)
    Image.open(g["ASSETS"] / "town.png").crop((0, 0, 928, 192)).save(root / "backgrounds/street.png")

    def obj(oid, name, kind, x, y, width=0, height=0, **props):
        return dict(id=oid, name=name, type=kind, x=x, y=y, width=width, height=height,
                    rotation=0, visible=True, point=width == height == 0,
                    properties=[prop(k, v) for k, v in props.items()])

    town_objects = [obj(1, "arrival", "spawn", 244, 208), obj(2, "from_shop", "spawn", 965, 208),
        obj(3, "tackle_entrance", "portal", 937, 176, 56, 32, target_map="yapshire:tackle_shop", target_spawn="arrival"),
        obj(4, "pier", "fishing", 1304, 176, 44, 32)]
    shop_objects = [obj(1, "arrival", "spawn", 96, 208),
        obj(2, "exit", "portal", 32, 176, 64, 32, target_map="yapshire:town", target_spawn="from_shop"),
        obj(3, "counter", "shop", 212, 176, 116, 32)]
    for name, objects in [("town", town_objects), ("tackle-shop", shop_objects)]:
        data = deepcopy(g["maps"][name])
        data["tilesets"] = references
        data["properties"] = [prop("yapshire:palette", json.dumps(palette, sort_keys=True, separators=(",", ":")))]
        for layer in data["layers"]:
            layer["data"] = [gids[names[gid]] if gid else 0 for gid in layer["data"]]
        data["layers"][2]["properties"] = [prop("collision", True)]
        if name == "town":
            data["layers"].append(dict(id=6, name="Street backdrop", type="imagelayer", x=0, y=0,
                offsetx=0, offsety=32, visible=True, opacity=1, image="../backgrounds/street.png",
                properties=[dict(name="z", type="float", value=-20)]))
        data["layers"].append(dict(id=7, name="Interactions", type="objectgroup", x=0, y=0,
                                   visible=True, opacity=1, objects=objects))
        data["nextlayerid"], data["nextobjectid"] = 8, len(objects) + 1
        write("maps/" + name + ".tmj", data)

    manifest = dict(format=1, id="yapshire", version="1.1.0", tile_size=16,
        tilesets=[group + ".tsj" for group in groups], images=["backgrounds/street.png"],
        maps=[dict(id="yapshire:town", path="maps/town.tmj", title="Harbor town", indoors=False),
              dict(id="yapshire:tackle_shop", path="maps/tackle-shop.tmj", title="Tide & Tackle", indoors=True)],
        entry=dict(map="yapshire:town", spawn="arrival"), prefabs=prefabs,
        terrains={"yapshire:quay": terrain_ids}, legacy_tiles=[names[i] for i in range(1, 360)])
    write("pack.json", manifest)
    # Every old tile has a stable identity, independently of the new atlas order.
    assert len(gids) == 375 and all(key in gids for key in manifest["legacy_tiles"])
    print(f"Wrote {root}: 4 tilesets, {len(gids)} stable tile IDs, {len(prefabs)} prefabs.")
