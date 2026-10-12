use yapshire_shared::{
    World,
    construction::{Catalog, Collision, Layer},
    terrain::{Block, TILE, Terrain},
};

#[test]
fn supplied_scenes_keep_editable_models_and_the_published_construction_catalog() {
    let world = World::bundled();
    let catalog = Catalog::new(&world.content);
    for info in &world.content.manifest.maps {
        let map = &world.maps[&info.id];
        assert!(
            map.layers.iter().all(|l| l.image.is_none()),
            "{} still flattens objects into an image",
            info.id
        );
        for layer in &map.layers {
            for &gid in layer.data.iter().filter(|&&g| g != 0) {
                let (_, _, tile) = world.content.resolve(map, &info.path, gid).unwrap();
                let piece = catalog
                    .get(tile.key())
                    .expect("Every scene tile is buildable");
                assert_eq!(piece.index, tile.id);
            }
        }
    }
    let town = &world.maps["yapshire:town"];
    let cedar = catalog
        .pieces
        .iter()
        .filter(|p| p.id.starts_with("yapshire:cedar/"))
        .filter(|p| world.content.gid(&p.id).is_some())
        .count();
    assert!(
        cedar == 8,
        "Published construction pieces must remain available to existing saves"
    );
    assert!(town.objects().any(|o| {
        o.block(&world.content)
            .is_some_and(|b| b.model("default", &world.content).parts.len() > 1)
    }));
    for name in [
        "cedar_tree",
        "birch_tree",
        "shrub",
        "cottage",
        "street_lamp",
        "garden_bench",
    ] {
        let prefab = &world.content.manifest.prefabs[&format!("yapshire:{name}")];
        assert!(
            prefab
                .tiles
                .iter()
                .filter(|id| !id.is_empty())
                .all(|id| catalog.get(id).is_some())
        );
    }
}

#[test]
fn independent_layers_survive_reload_and_obey_shared_collisions() {
    let catalog = Catalog::new(&World::bundled().content);
    let mut world = Terrain::new("Builder".into(), 42);
    let p = world.position;
    let (x, y) = (1027, 161);
    for (layer, id) in [
        (Layer::Wall, "yapshire:cedar"),
        (Layer::Solid, "yapshire:planks"),
        (Layer::Decor, "yapshire:flowers"),
    ] {
        assert!(world.place_tile(layer, x, y, id, p, &catalog));
        assert!(
            !world.place_tile(layer, x, y, id, p, &catalog),
            "Placing cannot silently replace a cell"
        );
    }
    world.validate_catalog(&catalog).unwrap();
    let mut restored: Terrain =
        serde_json::from_slice(&serde_json::to_vec(&world).unwrap()).unwrap();
    restored.validate_catalog(&catalog).unwrap();
    assert_eq!(restored, world);
    assert_eq!(restored.collision(x, y, &catalog), Collision::Solid);
    assert!(restored.dig_tile(Layer::Solid, x, y, p, &catalog));
    assert_eq!(restored.collision(x, y, &catalog), Collision::None);
    assert_eq!(restored.cell(Layer::Wall, x, y, &catalog), "yapshire:cedar");
    assert_eq!(
        restored.cell(Layer::Decor, x, y, &catalog),
        "yapshire:flowers"
    );
    assert!(restored.place_tile(Layer::Solid, x, y, "yapshire:platform", p, &catalog));
    assert_eq!(restored.collision(x, y, &catalog), Collision::Platform);
    assert!(!restored.place_tile(Layer::Solid, 1024, 161, "yapshire:planks", p, &catalog));
    assert!(
        !restored.place_tile(Layer::Wall, 1027, 155, "yapshire:cedar", p, &catalog),
        "No building through terrain"
    );
    assert!(!restored.place_tile(Layer::Decor, 1040, 161, "yapshire:flowers", p, &catalog));
    assert!(!restored.place_tile(Layer::Decor, 1025, 161, "missing:tile", p, &catalog));
}

#[test]
fn generated_leaves_and_trunks_are_removable_cells_with_persistent_deletions() {
    let catalog = Catalog::new(&World::bundled().content);
    let mut world = Terrain::new("Forest".into(), 42);
    for (layer, prefix) in [
        (Layer::Wall, "yapshire:cedar_tree/v2/"),
        (Layer::Decor, "yapshire:cedar_tree/v2/"),
    ] {
        let (x, y) = (50..1000)
            .flat_map(|x| (161..210).map(move |y| (x, y)))
            .find(|&(x, y)| world.cell(layer, x, y, &catalog).starts_with(prefix))
            .unwrap();
        let p = [(x as f32 + 0.5) * TILE, y as f32 * TILE];
        let original = world.cell(layer, x, y, &catalog).to_owned();
        assert_eq!(world.block(x, y), Block::Air);
        assert_eq!(world.collision(x, y, &catalog), Collision::None);
        assert!(world.dig_tile(layer, x, y, p, &catalog));
        assert!(world.cell(layer, x, y, &catalog).is_empty());
        let restored: Terrain =
            serde_json::from_slice(&serde_json::to_vec(&world).unwrap()).unwrap();
        assert!(
            restored.cell(layer, x, y, &catalog).is_empty(),
            "Trees must not regenerate after loading"
        );
        assert!(world.place_tile(layer, x, y, &original, p, &catalog));
        assert_eq!(world.cell(layer, x, y, &catalog), original);
    }
}

#[test]
fn woodland_recipes_are_versioned_without_regrowing_old_clearings() {
    let mut catalog = Catalog::new(&World::bundled().content);
    let mut old = Terrain::new("Old forest".into(), 42);
    old.vegetation = 1;
    let (x, y) = (50..1000)
        .flat_map(|x| (161..210).map(move |y| (x, y)))
        .find(|&(x, y)| old.cell(Layer::Decor, x, y, &catalog) == "yapshire:leaves/15")
        .unwrap();
    let p = [(x as f32 + 0.5) * TILE, y as f32 * TILE];
    assert!(old.dig_tile(Layer::Decor, x, y, p, &catalog));
    let restored: Terrain = serde_json::from_slice(&serde_json::to_vec(&old).unwrap()).unwrap();
    assert_eq!(restored.vegetation, 1);
    assert!(restored.cell(Layer::Decor, x, y, &catalog).is_empty());
    let new = Terrain::new("New forest".into(), 42);
    assert_eq!(new.vegetation, 2);
    new.validate_catalog(&catalog).unwrap();
    catalog.prefabs.remove("yapshire:cedar_tree/v2");
    old.validate_catalog(&catalog).unwrap();
    assert!(
        new.validate_catalog(&catalog)
            .unwrap_err()
            .to_string()
            .contains("cedar_tree/v2")
    );
}

#[test]
fn connected_foliage_updates_across_chunk_boundaries_and_after_erasing() {
    let catalog = Catalog::new(&World::bundled().content);
    let mut world = Terrain::new("Leaves".into(), 42);
    world.vegetation = 0;
    let p = [1024.5 * TILE, 162.0 * TILE];
    assert!(world.place_tile(Layer::Decor, 1023, 164, "yapshire:leaves/15", p, &catalog));
    assert!(world.place_tile(Layer::Decor, 1024, 164, "yapshire:leaves/15", p, &catalog));
    assert_eq!(
        world.visual_cell(Layer::Decor, 1023, 164, &catalog),
        "yapshire:leaves/2"
    );
    assert_eq!(
        world.visual_cell(Layer::Decor, 1024, 164, &catalog),
        "yapshire:leaves/8"
    );
    assert!(world.dig_tile(Layer::Decor, 1024, 164, p, &catalog));
    assert_eq!(
        world.visual_cell(Layer::Decor, 1023, 164, &catalog),
        "yapshire:leaves/0"
    );
}

#[test]
fn version_one_worlds_keep_their_terrain_without_new_trees_or_lost_edits() {
    let catalog = Catalog::new(&World::bundled().content);
    let original = serde_json::json!({"format":1,"generator":1,"name":"Existing","seed":42,"position":[16392.0,2576.0],"inventory":[1,0,0],"changes":{"328706":"air"}});
    let mut world: Terrain = serde_json::from_value(original).unwrap();
    world.validate_catalog(&catalog).unwrap();
    assert_eq!(world.vegetation, 0);
    assert_eq!(world.block(1026, 160), Block::Air);
    for x in 0..2048 {
        for y in 150..205 {
            assert_eq!(world.cell(Layer::Decor, x, y, &catalog), "");
        }
    }
    assert!(world.place_tile(
        Layer::Wall,
        1027,
        161,
        "yapshire:cedar",
        world.position,
        &catalog
    ));
    assert_eq!(world.format, 2);
    assert_eq!(world.inventory, [1, 0, 0]);
    world.validate_catalog(&catalog).unwrap();
    let before = world.clone();
    let mut incomplete = World::bundled().content;
    incomplete
        .sets
        .get_mut("buildings/modules.tsj")
        .unwrap()
        .tiles
        .retain(|t| t.key() != "yapshire:cedar");
    assert!(
        world
            .validate_catalog(&Catalog::new(&incomplete))
            .unwrap_err()
            .to_string()
            .contains("yapshire:cedar")
    );
    assert_eq!(world, before);
}

#[test]
fn collected_terrain_connects_to_construction_without_overwriting_placed_tiles() {
    let catalog = Catalog::new(&World::bundled().content);
    let mut world = Terrain::new("Bridge".into(), 42);
    let p = world.position;
    assert!(world.dig(1026, 160, p));
    assert!(world.place_tile(Layer::Solid, 1027, 163, "yapshire:planks", p, &catalog));
    assert!(!world.place(1027, 163, Block::Dirt, p, &catalog));
    assert!(world.place(1027, 164, Block::Dirt, p, &catalog));
    assert_eq!(
        world.cell(Layer::Solid, 1027, 163, &catalog),
        "yapshire:planks"
    );
    assert_eq!(world.block(1027, 164), Block::Dirt);
    assert_eq!(world.inventory, [0, 0, 0]);
    world.validate_catalog(&catalog).unwrap();
}
