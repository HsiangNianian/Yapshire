"""Rebuild editable foreground art; hills.png is a separate illustrated panorama."""

from pathlib import Path
import math
import random
from PIL import Image, ImageDraw, ImageFont

OUT = Path(__file__).resolve().parents[1] / "assets"
random.seed(27)


def save(im, name):
    im.save(OUT / name)
    assert im.getbbox(), name


def canvas(w, h, color=(0, 0, 0, 0)):
    im = Image.new("RGBA", (w, h), color)
    return im, ImageDraw.Draw(im)


# All shapes are drawn at their actual in-game resolution, without antialiasing.
sky, d = canvas(720, 405)
for y in range(405):
    t = min(y / 360, 1)
    a, b = (134, 175, 192), (208, 213, 199)
    color = tuple(round(a[i] * (1 - t) + b[i] * t) for i in range(3))
    d.line((0, y, 720, y), fill=color)
save(sky, "sky.png")

cloud, d = canvas(96, 24)
for x in range(4, 89, 3):
    y = 13 + int(math.sin(x / 13) * 3)
    d.rectangle((x, y, x + 5, y + 2), fill=(228, 230, 218, 92))
    d.line((x + 1, y - 2, x + 7, y - 2), fill=(242, 230, 202, 68))
save(cloud, "cloud.png")

town, d = canvas(1440, 192)
GROUND = 175
font_path = OUT / "fonts/fusion-pixel.ttf"
font = ImageFont.truetype(str(font_path), 12) if font_path.exists() else ImageFont.load_default()


def rect(box, color):
    d.rectangle(tuple(int(v) for v in box), fill=color)


def tree(x, y=175, size=1.0, autumn=False):
    rng = random.Random(int(x * 19 + size * 100))
    height = int(146 * size)
    top = y - height
    rect((x - 3, top + 18, x + 3, y), "#3a3730")
    rect((x, top + 18, x + 1, y), "#968b6a" if autumn else "#72664a")
    if autumn:
        rect((x - 1, top + 12, x + 1, y - 2), "#bdb69a")
        for yy in range(top + 18, y - 2, 9):
            d.line((x - 1, yy, x + rng.randint(0, 2), yy), fill="#5e6351")
        for branch in range(12):
            side = -1 if branch % 2 else 1
            cy = top + int((12 + branch * 7) * size)
            reach = int(rng.randint(12, 27) * size)
            cx = x + side * reach
            d.line((x, cy + int(28 * size), cx, cy), fill="#999d81", width=2)
            for _ in range(18):
                xx = cx + rng.randint(-10, 10) * size
                yy = cy + rng.randint(-13, 9) * size
                width = rng.randint(3, 9) * size
                color = rng.choice(["#78794c", "#9b8b4e", "#bca363", "#d2b576"])
                d.polygon([(xx, yy-2), (xx+width-2, yy-3), (xx+width, yy),
                           (xx+width-1, yy+3), (xx-2, yy+2)], fill=color)
                d.line((xx, yy, xx+width//2, yy), fill="#c6ab69")
    else:
        for tier in range(15):
            for side in (-1, 1):
                cy = top + int((9 + tier * 7 + rng.randint(-2, 3)) * size)
                reach = int((4 + tier * 2.4) * size * rng.uniform(.65, 1.12))
                tip = x + side * reach
                d.polygon([(x, cy-7), (tip-side*3, cy), (tip, cy-2),
                           (tip-side, cy+4), (x+side*reach//2, cy+7), (x, cy+4)],
                          fill=rng.choice(["#263d3b", "#2e443d", "#354b40"]))
                for needle in range(2, reach, 3):
                    xx = x + side * needle
                    yy = cy + rng.randint(-3, 2)
                    d.line((xx, yy-3, xx+side*2, yy+1), fill="#425849")
                    if rng.random() < .55:
                        d.line((xx, yy, xx+side*3, yy), fill="#667754")
        d.line((x, top, x-1, top+13), fill="#687354")
    rect((x - 7, y - 2, x + 8, y), "#334538")


def bush(x, y, width=30):
    for dx in range(0, width, 3):
        height = random.randint(7, 16)
        d.polygon([(x+dx-6, y), (x+dx-2, y-height), (x+dx+4, y-3), (x+dx+8, y)], fill="#344b40")
        d.line((x+dx, y-1, x+dx-1, y-height), fill="#637b55")
        for yy in range(y-height+3, y, 3):
            d.line((x+dx-3, yy-2, x+dx+2, yy), fill=random.choice(["#82915f", "#495f44", "#a49b64"]))


def window(x, y, w=24, h=28):
    rect((x-2, y-2, x+w+2, y+h+2), "#755d50")
    rect((x, y, x+w, y+h), "#eeb97c")
    rect((x+2, y+2, x+w-2, y+h-2), "#ffda91")
    rect((x+3, y+3, x+6, y+h-3), "#ffeac2")
    rect((x+w//2, y, x+w//2+1, y+h), "#987355")
    rect((x, y+h//2, x+w, y+h//2+1), "#987355")
    rect((x-4, y+h+3, x+w+4, y+h+5), "#556c5c")


def house(x, width=128, roof="#344a48", wall="#91755a", label="COFFEE"):
    y, top = 175, 79
    rect((x+6, top+12, x+width-6, y), "#7a7160")
    rect((x+8, top+16, x+width-9, y-5), wall)
    rect((x+width-20, top+15, x+width-9, y-4), "#574c3e")
    for yy in range(top+24, y-6, 5):
        rect((x+9, yy, x+width-21, yy), "#61513f")
        rect((x+9, yy+1, x+width-21, yy+1), "#b2946a")
    rect((x+width-29, top-36, x+width-17, top-4), "#98705b")
    rect((x+width-31, top-36, x+width-15, top-32), "#cfaa81")
    d.polygon([(x-4, top+18), (x+21, top-21), (x+width-28, top-21), (x+width+3, top+18)], fill="#684f48")
    d.polygon([(x-1, top+12), (x+22, top-23), (x+width-28, top-23), (x+width, top+12)], fill=roof)
    for yy in range(top-20, top+13, 6):
        inset = int((top+13-yy)*0.68)
        rect((x+inset, yy, x+width-inset, yy+1), "#687469")
        for xx in range(x+inset+3, x+width-inset, 13):
            rect((xx, yy+2, xx+1, yy+4), "#293e3d")
    rect((x-3, top+14, x+width+3, top+18), "#6a524a")
    window(x+19, 119, 27, 28)
    window(x+width-47, 119, 27, 28)
    rect((x+width//2-12, 125, x+width//2+12, y-4), "#5e6558")
    rect((x+width//2-9, 129, x+width//2+9, y-7), "#9a9d79")
    rect((x+width//2-6, 132, x+width//2+6, 149), "#edcb8e")
    rect((x+width//2+6, 157, x+width//2+7, 158), "#ffe2a2")
    rect((x+width//2-16, y-4, x+width//2+16, y), "#c1b697")
    rect((x+20, 98, x+width-20, 113), "#536c5c")
    rect((x+22, 100, x+width-22, 100), "#95a57b")
    d.text((x+width//2, 99), label, font=font, fill="#f8dfac", anchor="mt")
    for wx in [x+17, x+width-47]:
        rect((wx-3, 153, wx+30, 161), "#9d7157")
        for fx in range(wx, wx+29, 5):
            rect((fx, 148, fx+1, 154), "#527557")
            rect((fx-1, 147, fx+2, 149), random.choice(["#d18e79", "#ead1a0", "#f4b783"]))


def lamp(x):
    rect((x-2, 121, x+1, 175), "#455557")
    rect((x-5, 173, x+4, 175), "#485857")
    rect((x-5, 113, x+5, 125), "#465154")
    rect((x-3, 115, x+3, 122), "#ffdfa1")
    rect((x-7, 111, x+7, 113), "#596556")
    rect((x-3, 108, x+3, 110), "#596556")


def bench(x):
    for y in [151, 155, 163]:
        rect((x, y, x+39, y+2), "#92705a")
        rect((x, y, x+39, y), "#c49b70")
    for xx in [x+4, x+34]:
        rect((xx, 150, xx+2, 175), "#536153")


# Fence and planting form a continuous walking route.
for x in range(0, 1440, 22):
    rect((x, 157, x+2, 174), "#5c6250")
    rect((x, 158, x, 172), "#a59772")
rect((0, 162, 1440, 163), "#696d56")
for x in [15, 94, 331, 426, 530, 758, 868, 1074, 1291, 1390]:
    bush(x, 173, random.randint(25, 49))
for args in [(37, 175, 1.15, False), (383, 175, .94, True), (511, 175, .82, False),
             (855, 175, 1.15, False), (1011, 175, .9, True), (1370, 175, 1.3, False)]:
    tree(*args)
house(144, 152, label="COFFEE")
house(602, 132, roof="#465452", wall="#88765c", label="FLOWERS")
house(1123, 135, roof="#56605b", wall="#877864", label="POST")
for x in [103, 334, 561, 780, 1085, 1291]:
    lamp(x)
for x in [431, 914, 1300]:
    bench(x)
for x in [128, 304, 583, 741, 1105, 1261]:
    rect((x-4, 162, x+5, 174), "#ba8864")
    rect((x-6, 160, x+7, 163), "#d3a178")
    bush(x-3, 161, 6)
# Cafe awning, outdoor table, chalkboard and pennants.
for i in range(12):
    x = 146 + i*12
    d.polygon([(x, 114), (x+11, 114), (x+14, 125), (x-2, 125)], fill="#a9a083" if i%2 else "#435f50")
rect((144, 125, 291, 127), "#4a665b")
rect((306, 149, 324, 151), "#b18d66")
rect((314, 152, 316, 175), "#7a7259")
rect((310, 146, 313, 148), "#fff0c8")
rect((119, 151, 135, 174), "#9b7b59")
rect((121, 153, 133, 169), "#3b5953")
for y in [157, 161, 165]:
    rect((124, y, 130, y), "#ded7a8")
d.line((297, 98, 380, 119, 425, 98), fill="#56695a", width=1)
for x in range(304, 425, 13):
    y = int(99 + (x-297)*.25) if x<380 else int(119-(x-380)*.47)
    d.polygon([(x, y), (x+8, y+2), (x+4, y+10)], fill=random.choice(["#e2b277", "#cf8f70", "#99a484"]))
# Flower shop crates and a bicycle near the post office.
for x in range(756, 780, 10):
    rect((x, 164, x+8, 174), "#92705c")
    for j in range(3):
        rect((x+j*3, 152+j, x+j*3+1, 164), "#618261")
        d.ellipse((x+j*3-2, 149+j, x+j*3+3, 153+j), fill="#e7ba88")
for x in [1272, 1291]:
    d.ellipse((x-6, 161, x+6, 174), outline="#455a59", width=2)
d.line((1272, 167, 1280, 154, 1288, 167, 1272, 167, 1285, 157, 1291, 167), fill="#b8735e", width=2)
d.line((1284, 156, 1287, 151, 1292, 151), fill="#455a59", width=1)
for x in range(0, 1440, 4):
    if random.random() < .37:
        y = random.randint(170, 176)
        rect((x, y, x, 176), random.choice(["#567b57", "#96a06a", "#bdba81"]))
save(town, "town.png")

sheet, d = canvas(20*6, 32*4)
for row, (coat, light, hair, hat) in enumerate([
    ("#78573c", "#b09262", "#353a33", "#b9ab86"),
    ("#815049", "#ba8270", "#3b3c35", "#c7bca0"),
    ("#34574c", "#67836a", "#6d5140", "#ae9d76"),
    ("#40586a", "#7d9398", "#aaa389", "#5c706f"),
]):
    for frame in range(6):
        ox, oy = frame*20, row*32
        bob = 1 if frame in [2, 4] else 0

        def r(box, color):
            x1, y1, x2, y2 = box
            d.rectangle((ox+x1, oy+y1+bob, ox+x2, oy+y2+bob), fill=color)

        stride = [0, 0, -2, -1, 2, 1][frame]
        for lx, offset, color in [(8, stride, "#34494b"), (12, -stride, "#4b6263")]:
            d.line((ox+lx, oy+21, ox+lx+offset, oy+29), fill=color, width=2)
            d.line((ox+lx+offset-1, oy+30, ox+lx+offset+2, oy+30), fill="#2b3939", width=2)
            d.point((ox+lx+offset+2, oy+30), fill="#97896b")
        r((6, 12, 13, 21), coat)
        r((8, 12, 12, 18), light)
        r((11, 14, 11, 20), "#c0ac83")
        r((5, 14, 7, 20), "#6b5c43")
        r((4, 14, 6, 18), "#98815b")
        arm = [0, 0, 1, 0, -1, 0][frame]
        r((12+arm, 14, 14+arm, 19), coat)
        r((13+arm, 19, 14+arm, 21), "#ceaa83")
        r((7, 5, 12, 11), hair)
        r((9, 6, 14, 10), "#d9b48a")
        r((13, 9, 15, 10), "#d9b48a")
        r((13, 7, 13, 7), "#273732")
        r((10, 11, 12, 12), "#b98c6e")
        r((6, 4, 14, 5), hat)
        r((8, 2, 12, 4), hat)
        r((9, 2, 12, 2), "#d0bd92")
        r((6, 5, 16, 5), "#867654")
        r((8, 12, 12, 12), "#7d9988" if row == 0 else "#cabd99")
        if row == 1:
            r((8, 15, 11, 21), "#b8ae8c")
        elif row == 2:
            r((8, 19, 12, 21), "#35534a")
save(sheet, "people.png")

shadow, d = canvas(16, 4)
d.ellipse((0, 0, 15, 3), fill=(44, 63, 52, 72))
save(shadow, "shadow.png")

assert sheet.size == (120, 128)
assert sheet.crop((0, 0, 20, 32)).tobytes() != sheet.crop((40, 0, 60, 32)).tobytes()
assert town.size == (1440, 192)
print("Wrote 5 foreground pixel-art assets; dimensions and walk frames verified. Panorama preserved.")
