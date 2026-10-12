//! Keyed UI messages. English is the source catalog; translations are optional.
use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
    sync::OnceLock,
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum Language {
    #[default]
    #[serde(rename = "en")]
    English,
    #[serde(rename = "zh-CN")]
    Chinese,
}

impl Language {
    pub const ALL: [Self; 2] = [Self::English, Self::Chinese];

    pub fn native_name(self) -> &'static str {
        match self {
            Self::English => "English",
            Self::Chinese => "简体中文",
        }
    }
}

#[derive(Resource, Default)]
pub(crate) struct I18n {
    pub language: Language,
}

type Catalog = BTreeMap<String, String>;

// Register new domains here. Keeping English embedded guarantees a fallback in
// every native build, without depending on the working directory or a network.
const DOMAINS: [(&str, &str, &str); 10] = [
    (
        "pixels",
        include_str!("../assets/locales/en/pixels.json"),
        include_str!("../assets/locales/zh-CN/pixels.json"),
    ),
    (
        "tiles",
        include_str!("../assets/locales/en/tiles.json"),
        include_str!("../assets/locales/zh-CN/tiles.json"),
    ),
    (
        "wilds",
        include_str!("../assets/locales/en/wilds.json"),
        include_str!("../assets/locales/zh-CN/wilds.json"),
    ),
    (
        "clubs",
        include_str!("../assets/locales/en/clubs.json"),
        include_str!("../assets/locales/zh-CN/clubs.json"),
    ),
    (
        "common",
        include_str!("../assets/locales/en/common.json"),
        include_str!("../assets/locales/zh-CN/common.json"),
    ),
    (
        "menu",
        include_str!("../assets/locales/en/menu.json"),
        include_str!("../assets/locales/zh-CN/menu.json"),
    ),
    (
        "settings",
        include_str!("../assets/locales/en/settings.json"),
        include_str!("../assets/locales/zh-CN/settings.json"),
    ),
    (
        "editor",
        include_str!("../assets/locales/en/editor.json"),
        include_str!("../assets/locales/zh-CN/editor.json"),
    ),
    (
        "fishing",
        include_str!("../assets/locales/en/fishing.json"),
        include_str!("../assets/locales/zh-CN/fishing.json"),
    ),
    (
        "world",
        include_str!("../assets/locales/en/world.json"),
        include_str!("../assets/locales/zh-CN/world.json"),
    ),
];

fn catalogs() -> &'static [Catalog; 2] {
    static CATALOGS: OnceLock<[Catalog; 2]> = OnceLock::new();
    CATALOGS.get_or_init(|| {
        let mut catalogs = [Catalog::new(), Catalog::new()];
        for (domain, english, chinese) in DOMAINS {
            for (index, source) in [english, chinese].into_iter().enumerate() {
                let entries: BTreeMap<String, serde_json::Value> = serde_json::from_str(source)
                    .unwrap_or_else(|error| {
                        if index == 0 {
                            panic!("Invalid English catalog {domain}: {error}");
                        }
                        warn!("Ignoring invalid translation catalog {domain}: {error}");
                        BTreeMap::new()
                    });
                for (key, value) in entries {
                    if let Some(value) = value.as_str() {
                        catalogs[index].insert(format!("{domain}.{key}"), value.into());
                    }
                }
            }
        }
        catalogs
    })
}

fn placeholders(template: &str) -> BTreeSet<&str> {
    template
        .split('{')
        .skip(1)
        .filter_map(|part| part.split_once('}').map(|(key, _)| key))
        .collect()
}

fn lookup<'a>(key: &'a str, english: &'a Catalog, translation: Option<&'a Catalog>) -> &'a str {
    let Some(source) = english.get(key) else {
        return key;
    };
    translation
        .and_then(|catalog| catalog.get(key))
        .filter(|value| !value.trim().is_empty() && placeholders(value) == placeholders(source))
        .map(String::as_str)
        .unwrap_or(source)
}

impl I18n {
    pub fn text<'a>(&self, key: &'a str) -> &'a str {
        let catalogs = catalogs();
        lookup(
            key,
            &catalogs[0],
            (self.language == Language::Chinese).then_some(&catalogs[1]),
        )
    }
}

/// Literal player text stays literal; only explicit `tr` calls look up a key.
/// Arguments can themselves be messages (fish/map names), so stored notices
/// change language along with the rest of the UI.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Message {
    Literal(String),
    Key {
        key: &'static str,
        args: Vec<(&'static str, Message)>,
    },
}

pub(crate) fn tr(key: &'static str) -> Message {
    Message::Key {
        key,
        args: Vec::new(),
    }
}

impl Default for Message {
    fn default() -> Self {
        Self::Literal(String::new())
    }
}

impl From<String> for Message {
    fn from(value: String) -> Self {
        Self::Literal(value)
    }
}
impl From<&str> for Message {
    fn from(value: &str) -> Self {
        Self::Literal(value.into())
    }
}
impl From<&String> for Message {
    fn from(value: &String) -> Self {
        Self::Literal(value.clone())
    }
}
impl From<&Message> for Message {
    fn from(value: &Message) -> Self {
        value.clone()
    }
}

impl Message {
    pub fn arg(mut self, key: &'static str, value: impl Into<Message>) -> Self {
        if let Self::Key { args, .. } = &mut self {
            args.push((key, value.into()));
        }
        self
    }
    pub fn is_empty(&self) -> bool {
        matches!(self, Self::Literal(value) if value.is_empty())
    }
    pub fn clear(&mut self) {
        *self = Self::default();
    }
    pub fn render(&self, i18n: &I18n) -> String {
        let Self::Key { key, args } = self else {
            let Self::Literal(value) = self else {
                unreachable!()
            };
            return value.clone();
        };
        let mut source = i18n.text(key);
        let mut output = String::new();
        // One pass: braces in a nickname/path are never interpreted as tokens.
        while let Some((before, rest)) = source.split_once('{') {
            output.push_str(before);
            let Some((name, after)) = rest.split_once('}') else {
                output.push('{');
                output.push_str(rest);
                return output;
            };
            if let Some((_, value)) = args.iter().find(|(key, _)| *key == name) {
                output.push_str(&value.render(i18n));
            } else {
                output.push('{');
                output.push_str(name);
                output.push('}');
            }
            source = after;
        }
        output.push_str(source);
        output
    }
}

impl fmt::Display for Message {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.render(&I18n::default()))
    }
}

#[derive(Component)]
pub(crate) struct Localized(pub Message);

pub(crate) fn refresh(
    i18n: Res<I18n>,
    mut ui: Query<(Ref<Localized>, &mut Text)>,
    mut world: Query<(Ref<Localized>, &mut Text2d)>,
) {
    for (message, mut text) in &mut ui {
        if !i18n.is_changed() && !message.is_changed() {
            continue;
        }
        let value = message.0.render(&i18n);
        if **text != value {
            **text = value;
        }
    }
    for (message, mut text) in &mut world {
        if !i18n.is_changed() && !message.is_changed() {
            continue;
        }
        let value = message.0.render(&i18n);
        if **text != value {
            **text = value;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn incomplete_or_invalid_translations_fall_back_to_english_per_key() {
        let english = Catalog::from([("hello".into(), "Hello, {name}!".into())]);
        for translated in [
            None,
            Some(""),
            Some("  "),
            Some("你好！"),
            Some("你好，{other}！"),
        ] {
            let catalog = translated
                .map(|value| Catalog::from([("hello".into(), value.into())]))
                .unwrap_or_default();
            assert_eq!(lookup("hello", &english, Some(&catalog)), "Hello, {name}!");
        }
        let chinese = Catalog::from([("hello".into(), "你好，{name}！".into())]);
        assert_eq!(lookup("hello", &english, Some(&chinese)), "你好，{name}！");
    }

    #[test]
    fn locale_switch_reformats_saved_messages_without_translating_player_text() {
        let message = tr("menu.joined").arg("name", "{name} 小风");
        assert_eq!(
            message.render(&I18n::default()),
            "{name} 小风 joined the town."
        );
        assert_eq!(
            message.render(&I18n {
                language: Language::Chinese
            }),
            "{name} 小风来到了小镇。"
        );
        assert_eq!(
            Message::from("menu.joined").render(&I18n::default()),
            "menu.joined"
        );
    }

    #[test]
    fn shipped_catalogs_have_valid_placeholders_and_english_sources() {
        let catalogs = catalogs();
        assert!(!catalogs[0].is_empty());
        for (key, translation) in &catalogs[1] {
            let english = catalogs[0]
                .get(key)
                .unwrap_or_else(|| panic!("Missing English source: {key}"));
            if translation.trim().is_empty() {
                continue;
            }
            assert_eq!(placeholders(english), placeholders(translation), "{key}");
        }
    }

    #[test]
    fn interface_keys_always_have_an_english_fallback() {
        let english = &catalogs()[0];
        let mut folders = vec![std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src")];
        while let Some(folder) = folders.pop() {
            for entry in std::fs::read_dir(folder).unwrap() {
                let path = entry.unwrap().path();
                if path.is_dir() {
                    folders.push(path);
                    continue;
                }
                if path.extension().is_none_or(|ext| ext != "rs") {
                    continue;
                }
                let source = std::fs::read_to_string(&path).unwrap();
                for rest in source.split("tr(\"").skip(1) {
                    let key = rest.split('"').next().unwrap();
                    assert!(
                        english.contains_key(key),
                        "Missing English fallback for {key} in {}",
                        path.display()
                    );
                }
            }
        }
    }
}
