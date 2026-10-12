"""Author a new cell, change its source, and verify stable generated assets."""
import json
from pathlib import Path
import tempfile
import unittest
from PIL import Image
from compile_pack import compile_pack


class CompilePackTests(unittest.TestCase):
    def fixture(self, root):
        def write(name, value):
            (root / name).write_text(json.dumps(value))
        Image.new("RGBA", (16, 16), (240, 100, 20, 255)).save(root / "leaf.png")
        write("leaf-model.json", dict(size=[16, 16], anchor=[0, 0], parts=[
            dict(texture="leaf.png", offset=[0, 0], size=[16, 16])]))
        write("leaf-block.json", dict(tile=True, model="test:leaf", colliders=[]))
        write("pack.json", dict(format=2, tilesets=[],
            blocks={"test:leaf": "leaf-block.json"}, models={"test:leaf": "leaf-model.json"}))
        (root / "map.tmj").write_bytes(b"map layouts must remain untouched")

    def test_sources_compile_without_changing_ids_or_maps(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            self.fixture(root)
            compile_pack(root)
            ts = json.loads((root / "generated/blocks.tsj").read_text())
            self.assertEqual(ts["tiles"][0]["id"], 0)
            self.assertEqual(ts["tilecount"], 1)
            self.assertEqual(Image.open(root / "generated/blocks.png").getpixel((8, 8)), (240, 100, 20, 255))
            block = json.loads((root / "leaf-block.json").read_text())
            block["colliders"] = [dict(rect=[0, 0, 16, 8], platform=True)]
            (root / "leaf-block.json").write_text(json.dumps(block))
            Image.new("RGBA", (16, 16), (10, 210, 40, 255)).save(root / "leaf.png")
            compile_pack(root)
            ts = json.loads((root / "generated/blocks.tsj").read_text())
            self.assertEqual(ts["tiles"][0]["id"], 0)
            self.assertEqual(ts["tilecount"], 1)
            self.assertIn(dict(name="collision", type="string", value="platform"), ts["tiles"][0]["properties"])
            self.assertEqual(Image.open(root / "generated/blocks.png").getpixel((8, 8)), (10, 210, 40, 255))
            before = {p: p.read_bytes() for p in root.rglob("*") if p.is_file()}
            compile_pack(root)
            self.assertEqual(before, {p: p.read_bytes() for p in root.rglob("*") if p.is_file()})
            self.assertEqual((root / "map.tmj").read_bytes(), b"map layouts must remain untouched")

    def test_invalid_texture_does_not_write_outputs(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            self.fixture(root)
            model = json.loads((root / "leaf-model.json").read_text())
            model["parts"][0]["texture"] = "../outside.png"
            (root / "leaf-model.json").write_text(json.dumps(model))
            before = (root / "pack.json").read_bytes()
            with self.assertRaisesRegex(ValueError, "escapes pack"):
                compile_pack(root)
            self.assertEqual((root / "pack.json").read_bytes(), before)
            self.assertFalse((root / "generated").exists())
