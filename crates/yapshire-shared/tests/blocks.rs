use yapshire_shared::{
    World,
    blocks::{Collider, Model, Part},
    maps::Map,
};

#[test]
fn scenery_reuses_native_structural_textures_and_tree_models() {
    let world = World::bundled();
    let content = &world.content;
    let pine = &content.models["yapshire:trees/pine"];
    let boughs: Vec<_> = pine
        .parts
        .iter()
        .filter(|p| p.texture.ends_with("pine_bough.png"))
        .collect();
    assert!(boughs.len() > 10);
    assert!(boughs.iter().all(|p| p.size == [32, 24]));
    let mut pines = 0;
    for layer in &world.maps["yapshire:town"].layers {
        for object in &layer.objects {
            let Some(block) = object.block(content) else {
                continue;
            };
            let model = block.model(object.state(), content);
            for part in &model.parts {
                assert!(!part.texture.contains("/part_"));
                let png = yapshire_shared::bundled_file(&part.texture).unwrap();
                let native = [
                    u32::from_be_bytes(png[16..20].try_into().unwrap()),
                    u32::from_be_bytes(png[20..24].try_into().unwrap()),
                ];
                assert_eq!(
                    part.size, native,
                    "One source pixel stays one world pixel in every instance"
                );
            }
            if object.property("block") == Some("yapshire:trees/pine") {
                pines += 1;
                assert_eq!(model, pine);
            }
        }
    }
    assert!(pines >= 2);
    let barrel = &content.models["yapshire:barrel"];
    let opened = &content.models["yapshire:barrel_open"];
    for (closed, open) in barrel.parts.iter().zip(&opened.parts) {
        assert_eq!(closed.texture, open.texture);
        if closed.texture.ends_with("staves.png") {
            assert_eq!(
                closed.offset[1] - barrel.anchor[1],
                open.offset[1] - opened.anchor[1]
            );
        } else {
            assert!(open.offset[1] - opened.anchor[1] < closed.offset[1] - barrel.anchor[1]);
        }
    }
}

#[test]
fn stock_scenes_reference_registered_sources_instead_of_a_baked_street() {
    let world = World::bundled();
    assert_eq!(world.content.manifest.format, 2);
    for info in &world.content.manifest.maps {
        let map = &world.maps[&info.id];
        for layer in &map.layers {
            assert_ne!(layer.name, "Street backdrop");
            for &gid in &layer.data {
                if gid == 0 {
                    continue;
                }
                let (_, _, tile) = world.content.resolve(map, &info.path, gid).unwrap();
                let id = yapshire_shared::maps::string_property(&tile.properties, "block").unwrap();
                assert!(world.content.blocks[id].tile);
            }
        }
        for object in map.objects().filter(|o| o.kind != "spawn") {
            assert!(object.block(&world.content).is_some(), "{}", object.name);
        }
    }
    let tree = &world.content.blocks["yapshire:trees/aspen"];
    assert!(tree.model("default", &world.content).parts.len() > 4);
    assert!(
        tree.colliders.is_empty(),
        "Background foliage does not make an invisible wall"
    );
}

#[test]
fn irregular_models_select_their_parts_and_use_independent_compound_collision() {
    let mut world = World::bundled();
    let id = "yapshire:barrel";
    let texture = world.content.models["yapshire:ground/0_0"].parts[0]
        .texture
        .clone();
    world.content.models.insert(
        id.into(),
        Model {
            size: [32, 32],
            anchor: [16, 32],
            parts: vec![
                Part {
                    texture: texture.clone(),
                    offset: [0, 0],
                    size: [8, 16],
                    flip_x: false,
                },
                Part {
                    texture,
                    offset: [24, 0],
                    size: [8, 16],
                    flip_x: true,
                },
            ],
        },
    );
    let block = world.content.blocks.get_mut(id).unwrap();
    block.colliders = vec![Collider {
        rect: [-2, -16, 4, 16],
        platform: false,
    }];
    let block = &world.content.blocks[id];
    assert!(block.hit("default", &world.content, -12.0, -24.0));
    assert!(block.hit("default", &world.content, 12.0, -24.0));
    assert!(
        !block.hit("default", &world.content, 0.0, -24.0),
        "The fork's empty gap stays empty"
    );
    let map = &world.maps["yapshire:town"];
    let instance = map
        .objects()
        .find(|o| o.property("block") == Some(id))
        .unwrap();
    assert!(map.block_surfaces(&world.content).contains(&(
        [
            instance.x - 2.0,
            map.origin_y() - instance.y,
            instance.x + 2.0,
            map.origin_y() - instance.y + 16.0
        ],
        true
    )));
    World::from_maps(world.content, world.maps).unwrap();
}

#[test]
fn model_references_states_bounds_and_missing_images_are_validated() {
    let world = World::bundled();
    let mut broken = world.content.clone();
    broken.blocks.get_mut("yapshire:ground/0_0").unwrap().tile = false;
    assert!(
        broken.validate().is_err(),
        "Terrain cannot reference object-only definitions"
    );
    let mut broken = world.content.clone();
    broken.models.get_mut("yapshire:barrel").unwrap().parts[0].texture = "missing.png".into();
    assert!(broken.validate().is_err());
    let mut broken = world.content.clone();
    broken.blocks.get_mut("yapshire:barrel").unwrap().colliders = vec![Collider {
        rect: [0, 0, 0, 32],
        platform: false,
    }];
    assert!(broken.validate().is_err());
    let mut broken = world.content.clone();
    broken
        .manifest
        .models
        .insert("yapshire:barrel".into(), "../outside.json".into());
    assert!(broken.validate().is_err());
    let mut world = world;
    let id = world.maps["yapshire:town"]
        .objects()
        .find(|o| o.property("block") == Some("yapshire:barrel"))
        .unwrap()
        .id;
    assert!(
        world
            .set_object_state("yapshire:town", id, "unknown")
            .is_err()
    );
    world.set_object_state("yapshire:town", id, "open").unwrap();
    world.validate().unwrap();
    let bytes = serde_json::to_vec(&world.maps["yapshire:town"]).unwrap();
    let restored: Map = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(
        restored.objects().find(|o| o.id == id).unwrap().state(),
        "open"
    );
}

#[test]
fn terrain_partial_collision_follows_tiled_flips_without_filling_transparency() {
    let mut world = World::bundled();
    world
        .content
        .blocks
        .get_mut("yapshire:ground/0_0")
        .unwrap()
        .colliders = vec![Collider {
        rect: [1, 2, 4, 6],
        platform: false,
    }];
    let map = world.maps.get_mut("yapshire:town").unwrap();
    for layer in &mut map.layers {
        layer.data.fill(0);
    }
    map.layers[2].data[13 * 90] = 0x8000_0001;
    assert_eq!(
        map.tile_surfaces(&world.content, "maps/town.tmj", 0, 13),
        vec![([11.0, -8.0, 15.0, -2.0], true)]
    );
}
