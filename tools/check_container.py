"""Run the actual image with a read-only custom town, password and two clients."""
import argparse
import json
from pathlib import Path
import shutil
import subprocess
import tempfile
import time
from check_server import check, http


def docker(*args):
    return subprocess.check_output(["docker", *args], text=True).strip()


def main(image, platform):
    options = ["--platform", platform] if platform else []
    docker("run", "--rm", *options, image, "--check")
    artifacts = Path(__file__).resolve().parent.parent / "artifacts"
    artifacts.mkdir(exist_ok=True)
    # Desktop Docker engines commonly share the checkout but not OS temp folders.
    with tempfile.TemporaryDirectory(prefix="container-", dir=artifacts) as directory:
        root = Path(directory)
        root.chmod(0o755)
        shutil.copytree(Path(__file__).resolve().parent.parent / "assets/packs/yapshire", root / "maps")
        town_file = root / "maps/maps/town.tmj"
        town = json.loads(town_file.read_text())
        town["layers"][4]["data"][500] = 0x8000005D
        town_file.write_text(json.dumps(town))
        shop = json.loads((root / "maps/maps/tackle-shop.tmj").read_text())
        (root / "server.json").write_text(json.dumps({"name": "Container town", "maps_dir": "maps"}))
        password = "container-fixture-password"
        container = docker("run", "--detach", *options, "--read-only", "--cap-drop", "ALL", "--security-opt", "no-new-privileges:true", "--publish", "127.0.0.1::4761", "--mount", f"type=bind,src={root},dst=/data,readonly", "--env", f"YAPSHIRE_SERVER_PASSWORD={password}", image)
        try:
            assert docker("exec", container, "id", "-u") == "10001"
            address = docker("port", container, "4761/tcp")
            for attempt in range(60):
                try:
                    http(address, "/health")
                    break
                except OSError:
                    if attempt == 59:
                        raise
                    time.sleep(0.5)
            check(address, password, {"town": town, "shop": shop})
            assert json.loads(town_file.read_text()) == town
            assert password not in docker("logs", container)
            docker("stop", "--time", "5", container)
            state = json.loads(docker("inspect", "--format", "{{json .State}}", container))
            assert state["ExitCode"] == 0 and not state["OOMKilled"], state
            print(f"PASS: {image} {platform or 'native'}, non-root, read-only maps, graceful shutdown")
        finally:
            docker("rm", "--force", container)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("image")
    parser.add_argument("--platform", default="")
    args = parser.parse_args()
    main(args.image, args.platform)
