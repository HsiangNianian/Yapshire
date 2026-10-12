"""Export the stock artwork into editable block/model/texture sources (no atlas coordinates)."""
import json
from pathlib import Path
from PIL import Image
from draw_models import build as draw_models


def build(root):
    root = Path(root)
    manifest = json.loads((root / "pack.json").read_text())
    if manifest["format"] != 1:
        raise ValueError("Stock conversion needs the baseline pack; run tools/draw_maps.py instead")
    manifest.update(format=2, version="2.0.0", blocks={}, models={})
    blocks, models, gids = {}, {}, {}

    def write(path, value):
        path = root / path
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(json.dumps(value, indent=2, ensure_ascii=False) + "\n")

    def texture(name, image):
        path = f"textures/{name}.png"
        (root / path).parent.mkdir(parents=True, exist_ok=True)
        image.save(root / path)
        return path

    def define(name, image=None, *, parts=None, size=None, anchor=None, **rules):
        key = "yapshire:" + name
        if image is not None:
            size = list(image.size)
            parts = [dict(texture=texture(name, image), offset=[0, 0], size=size)]
        models[key] = dict(size=size, anchor=anchor or [0, 0], parts=parts)
        blocks[key] = dict(model=key, **rules)
        return key

    first = 1
    for source in manifest["tilesets"]:
        ts = json.loads((root / source).read_text())
        atlas = Image.open(root / Path(source).parent / ts["image"]).convert("RGBA")
        for tile in ts["tiles"]:
            props = {p["name"]: p["value"] for p in tile["properties"]}
            key = props["id"]
            gids[key] = first + tile["id"]
            # Old object fragments stay in compatibility atlases, not the model/material library.
            if key.rsplit("/", 1)[0] in manifest["prefabs"]:
                continue
            x, y = tile["id"] % ts["columns"] * 16, tile["id"] // ts["columns"] * 16
            collision = props.get("collision", "none")
            colliders = [] if collision == "none" else [dict(rect=[0, 0, 16, 16], platform=collision == "platform")]
            material = "water" if key.startswith("yapshire:water/") else "solid" if collision == "solid" else None
            define(key.split(":", 1)[1], atlas.crop((x, y, x + 16, y + 16)), colliders=colliders, tile=True,
                   **({"material": material} if material else {}))
            tile["properties"] = [p for p in tile["properties"] if p["name"] != "block"] + [dict(name="block", type="string", value=key)]
        write(source, ts)
        first += ts["tilecount"]

    draw_models(define, models, root.parents[1] / "fonts/fusion-pixel.ttf")

    people = Image.open(root.parents[1] / "people.png").convert("RGBA")
    clerk = texture("people/shopkeeper", people.crop((0, 32, 20, 64)))
    counter = models["yapshire:counter"]
    counter["size"][1] += 16
    counter["anchor"][1] += 16
    for part in counter["parts"]:
        part["offset"][1] += 16
    counter["parts"].insert(0, dict(texture=clerk, offset=[42, 0], size=[20, 32]))

    # A shared behavior selects a model variant; the sprite pieces are still independently editable.
    blocks["yapshire:barrel"].update(behavior="toggle", interaction=[-32, -40, 64, 56], states={"open": {"model": "yapshire:barrel_open"}})
    blocks["yapshire:shop"].update(behavior="portal", interaction=[-31, -32, 56, 32], properties={"target_map": "yapshire:tackle_shop", "target_spawn": "arrival"})
    blocks["yapshire:door"].update(behavior="portal", interaction=[-32, -32, 64, 32], properties={"target_map": "yapshire:town", "target_spawn": "from_shop"})
    blocks["yapshire:counter"].update(behavior="shop", interaction=[-124, -32, 116, 32])
    define("fishing_spot", Image.new("RGBA", (1, 1)), behavior="fishing", interaction=[0, 0, 44, 32])
    for name, color in [("sand", (196, 164, 108)), ("water", (71, 124, 145))]:
        image = Image.new("RGBA", (32, 32))
        for y in range(32):
            for x in range(32):
                shade = ((x * 73856093) ^ (y * 19349663)) % 3 * 3
                image.putpixel((x, y), tuple(c + shade for c in color) + (255,))
        define("materials/" + name, image, anchor=[16, 32], material=name)

    def prop(key, value):
        return dict(name=key, type="float" if isinstance(value, (int, float)) else "string", value=value)

    next_id = 100
    def instance(key, x, y, **properties):
        nonlocal next_id
        next_id += 1
        return dict(id=next_id, name=key.split(":", 1)[1], type="block", x=x, y=y,
                    point=True, properties=[prop("block", key)] + [prop(k, v) for k, v in properties.items()])

    street = []
    for x in range(0, 928, 24):
        street.append(instance("yapshire:street/fence", x, 208))
    for x in [15, 94, 331, 426, 530, 758, 868]:
        street.append(instance("yapshire:plants/fern", x, 208))
    for x, kind in [(37, "pine"), (383, "aspen"), (511, "pine"), (855, "pine")]:
        street.append(instance("yapshire:trees/" + kind, x, 208))
    for x, name in [(220, "coffee"), (668, "flowers")]:
        street.append(instance("yapshire:street/" + name, x, 208))
    for x in [103, 334, 561, 780]:
        street.append(instance("yapshire:street/lamp", x, 208))
    for x in [431, 888]:
        street.append(instance("yapshire:street/bench", x, 208))
    for x in [128, 304, 583, 741]:
        street.append(instance("yapshire:street/pot", x, 208))
    for name, x, y in [("cafe_table", 324, 208), ("chalkboard", 112, 208), ("pennants", 302, 130)]:
        street.append(instance("yapshire:street/" + name, x, y))

    for info in manifest["maps"]:
        data = json.loads((root / info["path"]).read_text())
        next_id = max(next_id, max((o["id"] for l in data["layers"] for o in l.get("objects", [])), default=0))
        additions = []
        for depth, layer in enumerate(data["layers"]):
            if layer["type"] != "tilelayer":
                continue
            objects = []
            cells, width = layer["data"], data["width"]
            for key, prefab in manifest["prefabs"].items():
                pattern = [(i % prefab["width"], i // prefab["width"], gids[t]) for i, t in enumerate(prefab["tiles"]) if t]
                for y in range(data["height"] - prefab["height"] + 1):
                    for x in range(width - prefab["width"] + 1):
                        if all(cells[(y + dy) * width + x + dx] == gid for dx, dy, gid in pattern):
                            for dx, dy, _ in pattern:
                                cells[(y + dy) * width + x + dx] = 0
                            objects.append(instance(key, x * 16 + prefab["anchor"][0], y * 16 + prefab["anchor"][1]))
            if objects:
                additions.append(dict(name=layer["name"] + " objects", type="objectgroup", objects=objects,
                    properties=[dict(name="z", type="float", value=-30 + depth * 6)]))
        interactions = data["layers"][-1]
        interactions["objects"] = [o for o in interactions["objects"] if o["type"] == "spawn"]
        if info["id"] == "yapshire:town":
            interactions["objects"].append(instance("yapshire:fishing_spot", 1304, 176))
            data["layers"] = [l for l in data["layers"] if l["type"] != "imagelayer"]
            for layer in data["layers"]:
                if layer["name"] in ("Terrain", "Water"):
                    layer.setdefault("properties", []).append(dict(name="simulation", type="bool", value=True))
            additions.insert(0, dict(name="Street objects", type="objectgroup", objects=street,
                properties=[dict(name="z", type="float", value=-20)]))
        data["layers"][-1:-1] = additions
        for i, layer in enumerate(data["layers"]):
            layer["id"] = i + 1
        data["nextlayerid"], data["nextobjectid"] = len(data["layers"]) + 1, next_id + 1
        write(info["path"], data)

    for kind, entries in [("blocks", blocks), ("models", models)]:
        for key, definition in entries.items():
            path = f"{kind}/{key.split(':', 1)[1]}.json"
            manifest[kind][key] = path
            write(path, definition)
    # Background art remains a normal image resource; foreground objects no longer use it.
    write("pack.json", manifest)
    print(f"Authored {len(blocks)} blocks and {len(models)} models, including separate irregular foliage parts.")


if __name__ == "__main__":
    build(Path(__file__).resolve().parents[1] / "assets/packs/yapshire")
