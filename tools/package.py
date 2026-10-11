"""Bundle a native release with its assets and notices; no external packaging tools."""
import argparse
import json
from pathlib import Path
import plistlib
import shutil
import tarfile
import tomllib
import zipfile

TARGETS = {
    "x86_64-unknown-linux-gnu": "linux-x64",
    "x86_64-pc-windows-msvc": "windows-x64",
    "aarch64-apple-darwin": "macos-arm64",
    "x86_64-apple-darwin": "macos-x64",
}


def package(root, target, server=False):
    root = Path(root)
    platform = TARGETS[target]
    version = tomllib.loads((root / "Cargo.toml").read_text())["package"]["version"]
    product = "yapshire-server" if server else "yapshire"
    name = f"{product}-{version}-{platform}"
    windows, mac = "windows" in platform, "macos" in platform and not server
    binary = root / "target" / target / "release" / (product + (".exe" if windows else ""))
    if not binary.is_file() or not binary.stat().st_size:
        raise ValueError(f"Build {target} before packaging")
    stage = root / "dist" / name
    if stage.exists():
        shutil.rmtree(stage)
    stage.mkdir(parents=True)
    executable_dir = stage / "Yapshire.app/Contents/MacOS" if mac else stage
    executable_dir.mkdir(parents=True, exist_ok=True)
    shutil.copy2(binary, executable_dir / binary.name)
    (executable_dir / binary.name).chmod(0o755)
    if server:
        shutil.copytree(root / "assets/packs/yapshire", stage / "maps")
        (stage / "server.json").write_text(json.dumps({"name": "My Yapshire town", "room_code": "MAIN0001", "maps_dir": "maps"}, indent=2) + "\n")
    else:
        shutil.copytree(root / "assets", executable_dir / "assets")
    for file in ["README.md", "README.zh-CN.md", "LICENSE.md", "CHANGELOG.md"]:
        shutil.copy2(root / file, stage / file)
    if (root / "docs").is_dir():
        shutil.copytree(root / "docs", stage / "docs")
    pack_root = "maps" if server else "assets/packs/yapshire"
    manifest_path = executable_dir / pack_root / "pack.json"
    pack = json.loads(manifest_path.read_text())
    required_assets = [pack_root + "/pack.json"]
    required_assets += [pack_root + "/" + m["path"] for m in pack["maps"]]
    required_assets += [pack_root + "/" + p for p in pack["tilesets"] + pack["images"]]
    for source in pack["tilesets"]:
        tileset = json.loads((executable_dir / pack_root / source).read_text())
        required_assets.append(str(Path(pack_root) / Path(source).parent / tileset["image"]))
    if not server:
        required_assets += ["assets/people.png", "assets/fonts/fusion-pixel.ttf", "assets/fishing/items.png",
                            "assets/hills.png", "assets/sky.png", "assets/cloud.png", "assets/shadow.png",
                            "assets/fishing/frame.png", "assets/fishing/slot.png", "assets/fishing/water.png",
                            "assets/ui/editor-icons.png"]
    for required in required_assets:
        if not (executable_dir / required).is_file():
            raise ValueError(f"Missing bundled asset: {required}")
    if mac:
        with (stage / "Yapshire.app/Contents/Info.plist").open("wb") as file:
            plistlib.dump({"CFBundleExecutable": "yapshire", "CFBundleName": "Yapshire",
                          "CFBundleIdentifier": "io.github.hsiangnianian.yapshire",
                          "CFBundlePackageType": "APPL", "CFBundleVersion": version,
                          "CFBundleShortVersionString": version, "NSHighResolutionCapable": True}, file)
    archive = stage.with_name(name + (".zip" if windows else ".tar.gz"))
    if windows:
        with zipfile.ZipFile(archive, "w", zipfile.ZIP_DEFLATED) as file:
            for path in sorted(stage.rglob("*")):
                if path.is_file():
                    file.write(path, path.relative_to(stage.parent))
    else:
        with tarfile.open(archive, "w:gz") as file:
            file.add(stage, arcname=name)
    shutil.rmtree(stage)
    return archive


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--target", required=True, choices=TARGETS)
    parser.add_argument("--server", action="store_true", help="Package the standalone console server")
    args = parser.parse_args()
    print(package(Path(__file__).resolve().parent.parent, args.target, args.server))
