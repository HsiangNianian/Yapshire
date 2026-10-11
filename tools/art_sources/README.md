# Foreground source art

These four original RGBA sheets were generated with the built-in `image_gen`
tool, using `assets/hills.png` as the palette/detail reference. They are checked
in so that exporting the content pack requires no image service or API key.
The generator does not run in the game or in the server.

| Source | Contents | Runtime destination |
| --- | --- | --- |
| [architecture.png](architecture.png) | Cedar, slate, plaster, window, door, awning, cornice, chimney, sign | `assets/packs/yapshire/buildings/modules.png` |
| [woodland.png](woodland.png) | Spruce, birch, green and autumn shrubs | `assets/packs/yapshire/nature/garden.png` |
| [harbor.png](harbor.png) | Barrel, piling, waymark, rope, dock boards, quay masonry | Harbor/ground atlases and reusable waymark/rope cells |
| [props.png](props.png) | Lantern, bench, fence, planter, crate, board, grass, flowers, masonry | `assets/packs/yapshire/objects/street.png` and construction materials |

`tools/construction_tiles.py` defines the crop rectangles and target sizes. It
preserves alpha, downsamples once, then slices on the 16px grid. Gameplay uses
nearest sampling. Transparent fringe RGB is ignored for bounding-box measurement
but the original alpha is retained. Empty cells are omitted. The source sheets
are not loaded by the game; each placed cell points to a small atlas tile.

Facade recipes reuse siding, plaster, roof, trim and framed window/door modules.
A framed module keeps its wall texture aligned at cell boundaries. Roof/chimney
intersections are composited per cell. Town, the editor and the local building
catalog all consume the same tile IDs. Plants use individual branch/leaf cells,
with reusable connected foliage available separately for freeform construction.

The `cedar_tree/v2`, `birch_tree/v2`, `shrub/v2` and `autumn_shrub/v2` recipe
footprints are part of vegetation generator 2. Do not change their dimensions,
occupied cells or stable IDs after publishing. Introduce a new vegetation
version when changing their geometry; older saves must keep their deletions.

Rebuild with `python3 tools/draw_maps.py` (Pillow required). This replaces the
supplied pack maps, so keep player-authored packs outside the official directory.

## Prompt set

Tool: built-in `image_gen`; transparent background enabled for all four calls.
The briefs below record the subjects, style and production constraints. Source
outputs are preserved here; export sizing is specified by code, not by assuming
that the image model returns the requested dimensions.

### Woodland

Use case: stylized-concept. Production woodland sprite source sheet for a
side-view game. Image 1 (`assets/hills.png`) is the required detailed northern
autumn landscape style; image 2 (`assets/town.png`) supplies camera/proportions.
Four separate cutouts in a 2×2 layout: a mature irregular dark spruce with
asymmetric branches and porous needle silhouette; a golden autumn birch with
cream mottled bark and visible fine branches through the foliage; a broad low
moss-green shrub with ochre tips; and a golden autumn shrub with thin twigs.
Front orthographic elevation, no ground or shadow base. Fine crisp clusters at
the intended game scale, darker foreground, upper-left light. No neon colours,
thick outlines, geometric canopy, cartoon Christmas tree, labels or grid.
True alpha outside objects and inside canopy gaps.

### Architecture

Use case: stylized-concept. Modular foreground architecture for a side-view
pixel game matching `assets/hills.png`, with `assets/town.png` for proportions.
Nine separate elements in a 3×3 sheet, generous transparent margins. Top row:
cedar horizontal siding texture, dark blue-sage slate shingles, warm cream
plaster texture. Middle row: a framed cottage sash window, narrow sage shop
door with glazed upper panels and brass handle, wide sage/linen striped awning.
Bottom row: long detailed cedar cornice and brackets, red-brown brick chimney
with cap and moss, blank wide sage shop sign with brass/ochre trim and supports.
Front orthographic elevation, weathered materials, fine texture, upper-left
light. No perspective, floor, whole buildings, scenery, labels or text.

### Props

Use case: stylized-concept. A reusable foreground prop sprite sheet for a
detailed side-view pixel-art village game. `assets/hills.png` is the landscape
palette/detail reference; `architecture.png` provides the weathered materials.
Nine separate orthographic front-elevation objects with generous transparent
space in a 3×3 arrangement. Row 1: a tall slim cast-iron lantern and full post,
a wide slatted wooden park bench with curved iron legs, a short cedar picket
fence with two posts and rails. Row 2: a terracotta flower planter, a worn crate
with diagonal brace, a standing chalkboard with indistinct strokes and no
legible words. Row 3: fine meadow grass, tiny cream/ochre wildflowers, a square
weathered lake-stone masonry texture with moss in the joints. Subdued sage,
cool forest shadows, weathered cedar, ochre and cream; upper-left sunlight.
No floor, ground shadows, thick outlines, plastic props, saturated colours,
perspective, scenery, labels, grid or borders. Real transparency, including
holes between fence slats, bench parts and foliage.

### Harbor

Use case: stylized-concept. Production foreground harbor prop sheet, using
`assets/hills.png` for atmosphere/palette and `architecture.png` for material
detail. Six separated front-elevation objects in a 3×2 arrangement: weathered
cedar barrel with iron bands; slim wooden dock piling, capped, rope-wrapped and
water-worn; blank sage directional sign on a cedar post; shallow sagging mooring
rope; a square cedar dock-board texture; a square lake-stone masonry texture.
Upper-left light, cool green shadows, warm cream highlights, fine natural
clusters suitable for 16px tiles. No cartoon outlines, geometric placeholders,
perspective floor, ground shadows, scenery, labels, text, grid or overlap.
True transparent alpha. Game lettering is added with the bundled pixel font
while exporting; the reusable blank sign remains available separately.
