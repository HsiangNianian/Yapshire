"""Rebuild Tiled atlases from authored models. Existing stable IDs and map layouts stay intact."""
import argparse
import json
from pathlib import Path
from PIL import Image


def compile_pack(root):
    root = Path(root).resolve()

    def path(name):
        if not name or Path(name).is_absolute() or "\\" in name or ":" in name:
            raise ValueError(f"Invalid pack path: {name}")
        result = (root / name).resolve()
        if not result.is_relative_to(root):
            raise ValueError(f"Resource escapes pack: {name}")
        return result

    def read(name):
        return json.loads(path(name).read_text())

    pack = read("pack.json")
    blocks = {key: read(name) for key, name in pack.get("blocks", {}).items()}
    models = {key: read(name) for key, name in pack.get("models", {}).items()}
    known = set()
    sets = [(name, read(name)) for name in pack["tilesets"]]
    for _, tileset in sets:
        known.update(p["value"] for tile in tileset["tiles"] for p in tile["properties"] if p["name"] == "id")
    new = sorted(key for key, block in blocks.items() if key not in known and block.get("tile", False)
                 and models[block["model"]]["size"] == [16, 16]
                 and models[block["model"]]["anchor"] == [0, 0])
    if new:
        name = "generated/blocks.tsj"
        tileset = next((ts for source, ts in sets if source == name), None)
        if tileset is None:
            tileset = dict(type="tileset", name="Custom blocks", tilewidth=16, tileheight=16,
                           columns=16, margin=0, spacing=0, image="blocks.png", tiles=[])
            sets.append((name, tileset))
            pack["tilesets"].append(name)
        for key in new:
            block = blocks[key]
            collision = "none" if not block.get("colliders") else "platform" if all(c.get("platform") for c in block["colliders"]) else "solid"
            tileset["tiles"].append(dict(id=len(tileset["tiles"]), properties=[
                dict(name=n, type="string", value=v) for n, v in [("id", key), ("block", key), ("collision", collision)]]))

    outputs = []
    for source, tileset in sets:
        count = len(tileset["tiles"])
        width, height = tileset["columns"] * 16, ((count + tileset["columns"] - 1) // tileset["columns"]) * 16
        atlas_path = path((Path(source).parent / tileset["image"]).as_posix())
        atlas = Image.open(atlas_path).convert("RGBA") if atlas_path.exists() else Image.new("RGBA", (width, height))
        if atlas.size != (width, height):
            grown = Image.new("RGBA", (width, height))
            grown.paste(atlas, (0, 0))
            atlas = grown
        for tile in tileset["tiles"]:
            properties = {p["name"]: p["value"] for p in tile["properties"]}
            if "block" not in properties:
                continue
            block = blocks[properties["block"]]
            model = models[block["model"]]
            if not block.get("tile") or model["size"] != [16, 16] or model["anchor"] != [0, 0]:
                raise ValueError("Terrain tiles require a 16x16 model anchored at its top left; place larger models as objects")
            image = Image.new("RGBA", (16, 16))
            for part in model["parts"]:
                sprite = Image.open(path(part["texture"])).convert("RGBA").resize(tuple(part["size"]), Image.Resampling.NEAREST)
                if part.get("flip_x"):
                    sprite = sprite.transpose(Image.Transpose.FLIP_LEFT_RIGHT)
                image.alpha_composite(sprite, tuple(part["offset"]))
            atlas.paste(image, (tile["id"] % tileset["columns"] * 16, tile["id"] // tileset["columns"] * 16))
            collision = "none" if not block.get("colliders") else "platform" if all(c.get("platform") for c in block["colliders"]) else "solid"
            tile["properties"] = [p for p in tile["properties"] if p["name"] != "collision"] + [dict(name="collision", type="string", value=collision)]
        tileset.update(tilecount=count, imagewidth=width, imageheight=height)
        outputs.append((atlas_path, atlas, path(source), tileset))
    # Resolve and render every source before touching the generated files.
    for atlas_path, atlas, source, tileset in outputs:
        atlas_path.parent.mkdir(parents=True, exist_ok=True)
        source.parent.mkdir(parents=True, exist_ok=True)
        atlas.save(atlas_path)
        source.write_text(json.dumps(tileset, indent=2, ensure_ascii=False) + "\n")
    path("pack.json").write_text(json.dumps(pack, indent=2, ensure_ascii=False) + "\n")
    print(f"Compiled {len(sets)} atlases from {len(blocks)} authored blocks; maps unchanged.")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("pack", type=Path)
    compile_pack(parser.parse_args().pack)
