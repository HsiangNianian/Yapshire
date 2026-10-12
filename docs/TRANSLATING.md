# Translating Yapshire

Yapshire v0.4.0 supports English (`en`, the default) and Simplified Chinese
(`zh-CN`). Open the pixel gear **Settings** button from the menu, game or map
editor. Language changes take effect immediately and are saved on this computer.

## Catalogs

Each language has independent JSON files under `assets/locales/`:

| File | Content |
| --- | --- |
| `common.json` | Shared actions and connection states |
| `menu.json` | Menus, chat controls and connection notices |
| `clubs.json` | Saved Clubs, room directory, latency and subscription notices |
| `settings.json` | Settings and preference notices |
| `editor.json` | Map tools, tooltips, layer names and save notices |
| `fishing.json` | Items, shop, fishing controls and results |
| `pixels.json` | Material tools and brush controls (v0.9.0) |
| `world.json` | Live text in the scene |

Keys are stable identifiers, prefixed by the filename in code. For example,
`editor.status.saved` looks up `status.saved` in `editor.json`.

English is the complete source catalog. For Chinese, copy the matching English
key into `zh-CN/<module>.json` and translate its value. Missing or blank values
fall back to English for that individual key. A malformed translation file falls
back for its whole module; invalid value types or mismatched placeholders also
fall back to English. An unknown language preference starts in English.

```json
{
  "status.saved": "已保存{map}，现在可以在本机游玩。"
}
```

Keep every named placeholder, such as `{map}`, `{name}` or `{coins}`. Their order
may change. Use `\n` for a line break, and preserve keyboard names such as
Ctrl/Cmd, Esc, B and F2. Do not translate keys. Braces are reserved for placeholders.

The catalogs are embedded at build time, so release builds always have their
English fallback. Rebuild after changing JSON; a running release does not reload
edited catalog files. No network translation service is used.

## Adding interface text

Add the English entry first, then an optional Chinese translation. Use a keyed
message instead of an English literal:

```rust
use crate::i18n::tr;

ui::label(commands, parent, art, tr("settings.title"), 32.0, ui::INK);
let notice = tr("editor.status.saved").arg("map", kind.title());
```

`Message` keeps the key and its named arguments until rendering. This lets an
existing notice change language, including translated item or map names. Plain
strings are literal: nicknames, chat, addresses, paths and custom room/layer names
are never treated as translation keys or reinterpreted as placeholders.

Static labels use `Localized`; dynamic readouts render a message with the `I18n`
resource. Keep data saved in `.tmj` maps independent of the chosen language.
The pixel font already includes Chinese glyphs. Signs painted into bitmap art
and external server/system error details retain their original text.

To add another language, extend `Language`, its native name and `ALL` in
`src/i18n.rs`, register its catalog files there, and extend the locale lookup.
The settings choices are generated from `Language::ALL`; individual UI modules
do not need another language branch.

## Preferences and checks

The language is stored in `settings.json` in the normal platform Yapshire data
directory (on macOS: `~/Library/Application Support/Yapshire/settings.json`).
`YAPSHIRE_SETTINGS_DIR` selects a separate directory for tests. Settings use an
atomic write and preserve other preference fields. A write failure keeps the
selected language for the current run and displays a notice.

```sh
cargo test --locked i18n::tests
cargo test --locked settings::tests
cargo run --locked
```

Check both languages in the actual game, including menus, editor tooltips, the
satchel, the shop and notices containing player text. For the repeatable native
language check, use a fresh test folder; it includes an offline gameplay UI
fixture, not a multiplayer connectivity test:

```sh
cargo build --locked
YAPSHIRE_SETTINGS_DIR="$PWD/artifacts/language-check/settings" \
YAPSHIRE_MAP_DIR="$PWD/artifacts/language-check/maps" \
YAPSHIRE_SAVE_DIR="$PWD/artifacts/language-check/progress" \
YAPSHIRE_SMOKE=i18n ./target/debug/yapshire

YAPSHIRE_SETTINGS_DIR="$PWD/artifacts/language-check/settings" \
YAPSHIRE_MAP_DIR="$PWD/artifacts/language-check/maps" \
YAPSHIRE_SAVE_DIR="$PWD/artifacts/language-check/progress" \
YAPSHIRE_SMOKE=i18n-reload ./target/debug/yapshire
```

Screenshots are written to `artifacts/i18n-*.png`. The check covers live switching,
literal player text, unsaved editor drafts, modal input isolation, translated
inventory/shop views and preference persistence across a restart.

Use `YAPSHIRE_SMOKE=i18n-compact` with another fresh set of folders to repeat it
at 844 × 390. This also taps the top-right gear through a real Bevy touch event
and checks its 48-point minimum hit area. This is a desktop window simulation of
a phone-sized display, not an Android/iOS device test.
