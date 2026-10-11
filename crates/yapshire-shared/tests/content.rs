use std::{collections::BTreeMap, path::PathBuf};
use yapshire_shared::{
    World,
    content::{Content, GID_MASK, MapInfo},
    maps::Map,
};

#[test]
fn official_pack_and_legacy_saves_preserve_tile_identity() {
    let world = World::bundled();
    assert_eq!(world.content.sets.len(), 4);
    assert_eq!(
        world
            .content
            .sets
            .values()
            .map(|s| s.tilecount)
            .sum::<u32>(),
        375
    );
    assert_eq!(world.entry(), ("yapshire:town", [244.0, 0.0]));
    let migrated = World::new(
        serde_json::from_str(yapshire_shared::TOWN).unwrap(),
        serde_json::from_str(yapshire_shared::SHOP).unwrap(),
    )
    .unwrap();
    for (id, source) in [
        ("yapshire:town", yapshire_shared::TOWN),
        ("yapshire:tackle_shop", yapshire_shared::SHOP),
    ] {
        let legacy: Map = serde_json::from_str(source).unwrap();
        let map = &migrated.maps[id];
        let path = &world
            .content
            .manifest
            .maps
            .iter()
            .find(|m| m.id == id)
            .unwrap()
            .path;
        for (old, new) in legacy.layers.iter().zip(&map.layers) {
            assert_eq!(old.data.len(), new.data.len());
            for (&before, &after) in old.data.iter().zip(&new.data) {
                if before == 0 {
                    assert_eq!(after, 0);
                } else {
                    assert_eq!(before & !GID_MASK, after & !GID_MASK);
                    let tile = world.content.resolve(map, path, after).unwrap().2;
                    assert_eq!(
                        tile.key(),
                        world.content.manifest.legacy_tiles[(before & GID_MASK) as usize - 1]
                    );
                }
            }
        }
        assert_eq!(
            world.maps[id].objects().collect::<Vec<_>>(),
            map.objects().collect::<Vec<_>>()
        );
    }
    let round_trip: World =
        serde_json::from_slice(&serde_json::to_vec_pretty(&world).unwrap()).unwrap();
    round_trip.validate().unwrap();
    assert_eq!(round_trip, world);
    let mut altered = world;
    altered.maps.get_mut("yapshire:town").unwrap().layers[4].data[50] = 0x8000_0001;
    assert!(
        altered
            .validate()
            .unwrap_err()
            .to_string()
            .contains("revision")
    );
}

#[test]
fn tiled_gid_ranges_and_atlas_positions_do_not_define_resource_identity() {
    let world = World::bundled();
    let mut map = world.maps["yapshire:town"].clone();
    let path = "maps/town.tmj";
    map.layers[4].data[50] = 0xe000_0001;
    let expected = map.clone();
    let original = map.clone();
    for (i, reference) in map.tilesets.iter_mut().enumerate() {
        reference.firstgid = 1000 + i as u32 * 400;
    }
    for layer in &mut map.layers {
        for gid in &mut layer.data {
            if *gid == 0 {
                continue;
            }
            let (source, _, tile) = world.content.resolve(&original, path, *gid).unwrap();
            let set = world
                .content
                .manifest
                .tilesets
                .iter()
                .position(|s| s == source)
                .unwrap();
            *gid = (1000 + set as u32 * 400 + tile.id) | (*gid & !GID_MASK);
        }
    }
    map.tilesets.reverse();
    map.validate(&world.content, path).unwrap();
    world.content.normalize(&mut map, path).unwrap();
    assert_eq!(map, expected);
    assert_eq!(
        world
            .content
            .resolve(&map, path, 0xe000_0001)
            .unwrap()
            .2
            .key(),
        "yapshire:ground/0_0"
    );
    // Repacking an atlas changes its local index, but looking up an ID still finds it.
    let mut content = world.content.clone();
    let set = content.sets.get_mut("terrain/ground.tsj").unwrap();
    set.tiles[0].id = 1;
    set.tiles[1].id = 0;
    set.tiles.sort_by_key(|t| t.id);
    content.validate().unwrap();
    assert_eq!(content.gid("yapshire:ground/0_0"), Some(2));
    let rebuilt = World::from_maps(content, world.maps).unwrap();
    let gid = rebuilt.maps["yapshire:town"].layers[2].data[13 * 90];
    assert_eq!(
        gid, 2,
        "existing maps follow the saved stable identity when repacked"
    );
}

#[test]
fn authored_map_portals_prefabs_and_terrain_need_no_game_code() {
    let mut world = World::bundled();
    let mut annex = world.maps["yapshire:tackle_shop"].clone();
    let content = &mut world.content;
    content.manifest.maps.push(MapInfo {
        id: "community:annex".into(),
        path: "maps/annex.tmj".into(),
        title: "Annex".into(),
        indoors: true,
    });
    let objects = &mut annex.layers.last_mut().unwrap().objects;
    let mut stamp = objects[0].clone();
    stamp.id = 9;
    stamp.name = "spare_barrel".into();
    stamp.kind = "prefab".into();
    stamp.x = 176.0;
    stamp.y = 208.0;
    stamp.properties = serde_json::from_value(
        serde_json::json!([{"name":"prefab","type":"string","value":"yapshire:barrel"}]),
    )
    .unwrap();
    objects.push(stamp);
    world.maps.insert("community:annex".into(), annex);
    let town = world.maps.get_mut("yapshire:town").unwrap();
    let entrance = town
        .layers
        .last_mut()
        .unwrap()
        .objects
        .iter_mut()
        .find(|o| o.kind == "portal")
        .unwrap();
    entrance.x = 480.0;
    entrance
        .properties
        .iter_mut()
        .find(|p| p.name == "target_map")
        .unwrap()
        .value = "community:annex".into();
    let world = World::from_maps(world.content, world.maps).unwrap();
    assert!(
        world.maps["yapshire:town"]
            .interaction(965.0, 0.0)
            .is_none()
    );
    let door = world.maps["yapshire:town"].interaction(500.0, 0.0).unwrap();
    assert_eq!(door.property("target_map"), Some("community:annex"));
    assert_eq!(
        world.maps["community:annex"].spawn("arrival"),
        Some([96.0, 0.0])
    );
    let expanded = world.maps["community:annex"].expanded(&world.content);
    assert!(
        expanded
            .layers
            .last()
            .unwrap()
            .data
            .iter()
            .any(|gid| *gid != 0)
    );
    let variants = &world.content.manifest.terrains["yapshire:quay"];
    assert_eq!(variants.len(), 16);
    assert!(
        variants
            .iter()
            .all(|id| world.content.find(id).unwrap().2.collision() == "solid")
    );
    let mut bad = world.clone();
    bad.maps.remove("community:annex");
    assert!(World::from_maps(bad.content, bad.maps).is_err());
}

struct Folder(PathBuf);
impl Folder {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "yapshire-content-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&path).unwrap();
        for (name, bytes) in yapshire_shared::BUNDLED_FILES {
            let file = path.join(name);
            std::fs::create_dir_all(file.parent().unwrap()).unwrap();
            std::fs::write(file, bytes).unwrap();
        }
        Self(path)
    }
}
impl Drop for Folder {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn pack_fingerprints_are_portable_and_mismatches_name_the_resource() {
    let folder = Folder::new();
    let bundled = Content::bundled();
    for (name, _) in yapshire_shared::BUNDLED_FILES.iter().filter(|(name, _)| {
        name.ends_with(".json") || name.ends_with(".tsj") || name.ends_with(".tmj")
    }) {
        let path = folder.0.join(name);
        let text = std::fs::read_to_string(&path)
            .unwrap()
            .replace('\n', "\r\n");
        std::fs::write(path, text).unwrap();
    }
    let loaded = Content::load(&folder.0).unwrap();
    bundled.require(&loaded).unwrap();
    assert_eq!(World::load(&folder.0).unwrap(), World::bundled());
    let path = folder.0.join("objects/harbor.png");
    let mut image = std::fs::read(&path).unwrap();
    image.push(0);
    std::fs::write(path, image).unwrap();
    let altered = Content::load(&folder.0).unwrap();
    let message = altered.require(&bundled).unwrap_err().to_string();
    assert!(message.contains("objects/harbor.png"));
    assert!(message.contains(&format!("v{}", bundled.manifest.version)));
}

#[test]
fn invalid_paths_ids_gids_animations_and_interactions_are_rejected() {
    let world = World::bundled();
    for path in [
        "../../secret",
        "/tmp/private",
        "C:\\private",
        "../a/../../b",
    ] {
        assert!(yapshire_shared::content::relative("maps/town.tmj", path).is_err());
    }
    let mut bad = world.content.clone();
    bad.sets.get_mut("terrain/ground.tsj").unwrap().tiles[1].properties[0].value =
        "yapshire:ground/0_0".into();
    assert!(bad.validate().is_err());
    let mut bad = world.content.clone();
    bad.sets.get_mut("terrain/water.tsj").unwrap().tiles[0].animation[1].duration = 0;
    assert!(bad.validate().is_err());
    for gid in [0x8000_0000, 0x1000_0001, 99999] {
        let mut map = world.maps["yapshire:town"].clone();
        map.layers[0].data[0] = gid;
        assert!(map.validate(&world.content, "maps/town.tmj").is_err());
    }
    let mut map = world.maps["yapshire:town"].clone();
    map.layers[0].image = Some("../../outside.png".into());
    assert!(map.validate(&world.content, "maps/town.tmj").is_err());
    let mut map = world.maps["yapshire:town"].clone();
    map.tilesets[1].firstgid = 1;
    assert!(map.validate(&world.content, "maps/town.tmj").is_err());
    let mut maps = world.maps.clone();
    maps.get_mut("yapshire:town")
        .unwrap()
        .layers
        .last_mut()
        .unwrap()
        .objects[2]
        .properties[0]
        .value = "unknown:map".into();
    assert!(
        World::from_maps(world.content.clone(), maps)
            .unwrap_err()
            .to_string()
            .contains("Broken portal")
    );
    let maps: BTreeMap<String, Map> = BTreeMap::new();
    assert!(World::from_maps(world.content, maps).is_err());
}

#[cfg(unix)]
#[test]
fn symlinks_cannot_import_files_outside_the_pack() {
    let folder = Folder::new();
    std::os::unix::fs::symlink("/etc/passwd", folder.0.join("escape.png")).unwrap();
    assert!(yapshire_shared::content::pack_file(&folder.0, "escape.png").is_err());
}
