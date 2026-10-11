"""Build bilingual SVG branding from the game's font and editable sprites."""

from collections import defaultdict
from html import escape
from pathlib import Path
from PIL import Image, ImageDraw, ImageFont

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / "docs/readme"


def pixels(image):
    colors = defaultdict(list)
    image = image.convert("RGBA")
    for y in range(image.height):
        x = 0
        while x < image.width:
            color = image.getpixel((x, y))
            end = x + 1
            while end < image.width and image.getpixel((end, y)) == color:
                end += 1
            if color[3]:
                colors[color].append(f"M{x} {y}h{end-x}v1h{x-end}z")
            x = end
    return "".join(f'<path fill="#{r:02x}{g:02x}{b:02x}" d="{"".join(paths)}"/>'
                   for (r, g, b, _), paths in colors.items())


def lettering(value, size, x, y, color):
    font = ImageFont.truetype(str(ROOT / "assets/fonts/fusion-pixel.ttf"), size)
    bounds = font.getbbox(value)
    mask = Image.new("RGBA", (bounds[2], bounds[3] - bounds[1]))
    ImageDraw.Draw(mask).text((0, -bounds[1]), value, font=font, fill=color)
    return f'<g aria-label="{escape(value)}" transform="translate({x} {y})">{pixels(mask)}</g>'


def main():
    town = Image.open(ROOT / "assets/town.png")
    people = Image.open(ROOT / "assets/people.png")
    assert town.size == (1440, 192) and people.size == (120, 128)
    OUT.mkdir(parents=True, exist_ok=True)
    for chinese in (False, True):
        for dark in (False, True):
            background, ink = ("#172a28", "#f1e3c5") if dark else ("#e5e7d7", "#29453e")
            accent = "#d5b170" if dark else "#74603d"
            tagline = "秋日湖岸，好友相伴。" if chinese else "AUTUMN DAYS. GOOD COMPANY."
            detail = "散步，聊天，甩一竿。" if chinese else "Walk. Talk. Cast a line."
            body = [f'<rect width="1200" height="240" fill="{background}"/>',
                    '<path d="M0 0h1200v2H0z M0 238h1200v2H0z" fill="#647561"/>',
                    lettering(tagline, 16, 48, 36, accent),
                    lettering("YAPSHIRE", 72, 44, 76, ink),
                    lettering(detail, 20, 48, 174, ink)]
            for crop, x, y, scale in [
                (town.crop((0, 0, 80, 180)), 832, 26, 1),
                (town.crop((344, 0, 429, 180)), 1070, 26, 1),
                (people.crop((0, 0, 20, 32)), 960, 142, 2),
                (people.crop((0, 32, 20, 64)), 1010, 142, 2),
            ]:
                body.append(f'<g transform="translate({x} {y}) scale({scale})">{pixels(crop)}</g>')
            body.append('<path d="M808 212h350v2H808z M834 220h78v2h-78z M1040 220h110v2h-110z" fill="#819683"/>')
            title = f"Yapshire — {tagline} {detail}"
            svg = ('<svg xmlns="http://www.w3.org/2000/svg" width="1200" height="240" '
                   'viewBox="0 0 1200 240" role="img" aria-labelledby="title" shape-rendering="crispEdges">'
                   f'<title id="title">{escape(title)}</title>' + "".join(body) + '</svg>\n')
            name = "banner" + ("-dark" if dark else "") + ("-zh" if chinese else "") + ".svg"
            (OUT / name).write_text(svg)
    print("Wrote four bilingual, font-independent banners from current game assets.")


if __name__ == "__main__":
    main()
