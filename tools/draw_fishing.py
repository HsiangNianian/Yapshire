"""Build original fishing sprites and pixel UI: uv run --with Pillow tools/draw_fishing.py."""
from pathlib import Path
from PIL import Image, ImageDraw

OUT = Path(__file__).resolve().parents[1] / "assets/fishing"
OUT.mkdir(parents=True, exist_ok=True)
INK, LIGHT = "#263d40", "#e6d6ae"
sheet = Image.new("RGBA", (128, 128))


def icon(index, draw):
    im = Image.new("RGBA", (32, 32))
    draw(ImageDraw.Draw(im))
    assert im.getbbox(), index
    sheet.paste(im, ((index % 4) * 32, (index // 4) * 32))


def rod(d):
    d.line((5, 28, 12, 17, 19, 8, 26, 3), fill=INK, width=4)
    d.line((5, 27, 12, 17, 19, 8, 26, 3), fill="#cfa66a", width=2)
    d.line((7, 24, 14, 14, 20, 7, 26, 3), fill=LIGHT)
    d.line((5, 27, 9, 22), fill="#795b4a", width=3)
    d.ellipse((9, 20, 15, 26), fill=INK)
    d.ellipse((10, 21, 14, 25), fill="#afbaa0")
    d.point((12, 22), fill=LIGHT)
    d.line((26, 4, 27, 21), fill="#9cafaa")
    d.line((25, 21, 25, 24, 27, 25, 28, 23), fill=LIGHT)


def hook(d):
    d.ellipse((13, 3, 19, 9), outline=INK, width=2)
    d.ellipse((14, 4, 18, 8), outline="#d9e2c5")
    d.line((17, 9, 17, 23, 15, 27, 10, 28, 6, 24, 6, 18), fill=INK, width=4)
    d.line((17, 9, 17, 22, 15, 26, 10, 27, 7, 24, 7, 18), fill="#c1d3c0", width=2)
    d.line((18, 10, 18, 22, 16, 26), fill=LIGHT)
    d.polygon([(5, 19), (9, 15), (10, 21)], fill="#e7e5be")


def bait(d):
    d.rectangle((5, 13, 27, 27), fill=INK)
    d.rectangle((7, 14, 25, 26), fill="#6a9484")
    d.rectangle((8, 15, 10, 25), fill="#95b99a")
    d.ellipse((4, 10, 28, 16), fill=INK)
    d.ellipse((6, 10, 26, 14), fill="#d4bb87")
    d.rectangle((12, 18, 24, 23), fill="#edce9d")
    d.line((16, 9, 13, 6, 15, 3, 19, 4, 18, 9, 21, 11), fill="#865958", width=3)
    d.line((16, 8, 14, 6, 15, 4, 18, 4, 17, 9, 21, 11), fill="#e3a18b")
    d.line((14, 20, 17, 22, 20, 20, 23, 21), fill="#b37968")


def coin(d):
    d.ellipse((6, 7, 26, 28), fill="#8b6947")
    d.ellipse((4, 4, 25, 25), fill=INK)
    d.ellipse((5, 4, 24, 24), fill="#d7a355")
    d.ellipse((7, 5, 23, 21), outline="#f1d88e", width=2)
    d.line((11, 9, 16, 9, 12, 17, 17, 17), fill="#997449", width=2)
    d.line((8, 6, 14, 6), fill="#fff0bd")


def fish(d, species, silhouette=False):
    palettes = [("#738e94", "#c8c8b1", "#4a666e"), ("#647b5b", "#b6ba8d", "#3d564e"),
                ("#9a886c", "#d2c79e", "#626c55"), ("#b99152", "#e2c886", "#806440")]
    back, belly, fin = palettes[species]
    if silhouette:
        back = belly = fin = "#3e6970"
    top, bottom = [(12, 21), (10, 22), (9, 23), (8, 25)][species]
    d.polygon([(22, 15), (30, 9), (29, 25), (22, 20)], fill=INK if not silhouette else fin)
    d.polygon([(23, 16), (28, 12), (27, 22), (23, 19)], fill=fin)
    d.polygon([(12, top + 1), (17, top - 5), (20, top + 1)], fill=fin)
    d.polygon([(13, bottom - 1), (20, bottom + 4), (20, bottom - 1)], fill=fin)
    d.ellipse((3, top, 25, bottom), fill=INK if not silhouette else fin)
    d.ellipse((4, top + 1, 24, bottom - 1), fill=back)
    d.pieslice((5, top + 1, 23, bottom - 1), 0, 180, fill=belly)
    d.line((9, top + 2, 17, top + 2), fill=LIGHT if not silhouette else back)
    if not silhouette:
        for x in range(12, 24, 3):
            if species == 1:
                d.line((x, top + 2, x - 1, 16), fill=fin)
            else:
                d.point((x, 16 + x % 2), fill=fin)
        d.line((10, top + 4, 12, 17, 10, bottom - 2), fill=fin)
        d.rectangle((5, top + 3, 7, top + 5), fill=LIGHT)
        d.point((5, top + 4), fill=INK)
        d.point((3, 18), fill=belly)


def bobber(d):
    d.line((16, 3, 16, 10), fill=LIGHT)
    d.ellipse((12, 9, 20, 22), fill=INK)
    d.ellipse((13, 10, 19, 21), fill="#e7dcb5")
    d.rectangle((13, 11, 19, 15), fill="#d08363")
    d.line((14, 11, 14, 14), fill="#f4bc87")
    d.line((16, 22, 16, 27), fill="#7e6956")


def ripple(d):
    d.arc((1, 11, 30, 22), 5, 170, fill="#bfd5b9")
    d.arc((4, 12, 27, 20), 180, 345, fill="#91b6a6")


def splash(d):
    for x, y in [(3, 11), (8, 5), (22, 6), (28, 12)]:
        d.line((x, y, x + 1, y + 3), fill="#d4e5c4", width=2)
    d.line((5, 22, 10, 19, 14, 22, 18, 18, 26, 22), fill="#d4e5c4", width=2)


def creel(d):
    d.arc((7, 0, 24, 16), 180, 360, fill=INK, width=4)
    d.arc((8, 2, 23, 16), 180, 360, fill="#c3a274", width=2)
    d.rectangle((3, 10, 28, 28), fill=INK)
    d.rectangle((5, 12, 26, 27), fill="#b18f5f")
    for y in range(14, 27, 3):
        d.line((5, y, 26, y), fill="#d4b780")
    for x in range(8, 27, 5):
        d.line((x, 13, x, 26), fill="#987b54")
    d.rectangle((3, 9, 28, 12), fill="#dfc391")
    d.rectangle((14, 10, 18, 18), fill="#647962")
    d.rectangle((15, 13, 17, 15), fill=LIGHT)


def star(d):
    d.polygon([(16, 3), (19, 12), (28, 16), (19, 19), (16, 29), (12, 19), (3, 16), (12, 12)], fill="#c18a4e")
    d.polygon([(16, 6), (18, 14), (25, 16), (18, 18), (16, 26), (14, 18), (6, 16), (14, 14)], fill="#ffe4a0")


def bite(d):
    d.rectangle((6, 3, 25, 24), fill=INK)
    d.rectangle((8, 5, 23, 22), fill="#f5dfa5")
    d.polygon([(14, 23), (19, 23), (15, 29)], fill="#f5dfa5")
    d.rectangle((14, 8, 17, 14), fill="#ad644e")
    d.rectangle((14, 17, 17, 19), fill="#ad644e")


for i, draw in enumerate([rod, hook, bait, coin]):
    icon(i, draw)
for i in range(4):
    icon(4 + i, lambda d, i=i: fish(d, i))
for i, draw in [(8, bobber), (9, ripple), (10, splash), (11, creel),
                (12, lambda d: fish(d, 1, True)), (13, star), (14, bite), (15, hook)]:
    icon(i, draw)
sheet.save(OUT / "items.png")

for name, slot in [("frame.png", False), ("slot.png", True)]:
    im = Image.new("RGBA", (32, 32), "#25372f" if slot else "#182925")
    d = ImageDraw.Draw(im)
    for inset, color in [(1, "#786849"), (2, "#b09b6f"), (3, "#3e4a39"),
                          (5, "#445448" if slot else "#283b32"),
                          (6, "#394c40" if slot else "#1e2e2a")]:
        d.rectangle((inset, inset, 31 - inset, 31 - inset), fill=color)
    for x, y in [(3, 3), (27, 3), (3, 27), (27, 27)]:
        d.rectangle((x, y, x + 1, y + 1), fill="#cdb98d")
    # Store a 2x nearest copy so UI borders use the same pixel pitch as the world.
    im.resize((64, 64), Image.Resampling.NEAREST).save(OUT / name)

im = Image.new("RGBA", (128, 56))
d = ImageDraw.Draw(im)
for y in range(56):
    t = y / 55
    c = tuple(round(a + (b - a) * t) for a, b in zip((100, 140, 150), (27, 57, 71)))
    d.line((0, y, 127, y), fill=c)
d.line((0, 2, 127, 2), fill="#bad2b4")
for x in [9, 29, 73, 103]:
    d.line((x, 6 + x % 5, x + 9, 6 + x % 5), fill="#90b7ab")
for x in range(0, 128, 19):
    d.ellipse((x - 8, 51 + x % 3, x + 10, 61), fill="#476e65", outline="#6c917b")
for x in [5, 13, 111, 121]:
    d.line((x, 55, x - 2, 48, x + 2, 43, x, 37), fill="#648f73", width=2)
for x, y in [(25, 35), (27, 30), (96, 26), (100, 18)]:
    d.ellipse((x, y, x + 2, y + 3), outline="#99b9aa")
im.save(OUT / "water.png")
assert sheet.size == (128, 128)
assert len({sheet.crop((x, 32, x + 32, 64)).tobytes() for x in range(0, 128, 32)}) == 4
print("Wrote 16 item sprites, two pixel frames and the fishing water view.")
