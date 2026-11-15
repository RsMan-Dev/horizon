//! `native_level_light`, `native_level_biome` and `native_level_block_collision` answer for the
//! level their handle names, where `native_light`, `native_biome` and `native_block_collision`
//! answer for the one in place.
use super::super::*;
use horizon_client::chunk::Chunk;
use horizon_client::mod_api::NativeAuthority;

fn game() -> Game<'static> {
    let root = std::path::PathBuf::from(
        std::env::var_os("HORIZON_TEST_RESOURCES")
            .expect("set HORIZON_TEST_RESOURCES to prepared resources"),
    );
    let settings = Settings::default();
    let loaded = load_resources(
        root.clone(),
        None,
        &root,
        &settings,
        std::time::Instant::now(),
        false,
        true,
        &Progress::new(),
        &mut |_, _| unreachable!(),
    )
    .unwrap();
    let mut options = Options::parse(std::iter::empty()).unwrap().unwrap();
    options.resources = root.clone();
    options.execution_role = execution_role::ExecutionRole::IntegratedServer;
    let res: &'static _ = Box::leak(Box::new(loaded.resources));
    let mut game = Game::new(
        res,
        loaded.pack,
        loaded.gui.unwrap(),
        options,
        settings,
        Audio::silent(&root),
        None,
    )
    .unwrap();
    game.build_registries();
    game.native_lifecycle
        .begin_open(NativeAuthority::IntegratedServer);
    game
}

/// The overworld holds a stone block, in the biome `desert`, in a loaded chunk; the nether, in
/// place, has none loaded there.
#[test]
#[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
fn the_reads_answer_for_the_level_named_not_the_one_in_place() {
    let mut game = game();
    let desert = game.res.biomes().id("minecraft:desert");
    let stone = game.pack.assets.registry.default_state("minecraft:stone");
    let mut chunk = Chunk::empty(0, 0);
    chunk.set(1, 64, 1, stone.unwrap().0);
    for (qx, qz, qy) in
        (0..4).flat_map(|x| (0..4).flat_map(move |z| (0..96).map(move |y| (x, z, y))))
    {
        chunk.set_biome_cell(qx, qy, qz, desert);
    }
    game.world.chunks.insert((0, 0), Arc::new(chunk));
    let overworld = game.native_level("minecraft:overworld").unwrap();
    game.enter_level(None, Dim::Nether).unwrap();
    let nether = game.native_level("minecraft:the_nether").unwrap();
    let at = [1, 64, 1];

    // In place: the nether, where that chunk isn't loaded.
    assert!(game.native_block_collision(at).is_empty());
    assert_eq!(game.native_biome(at), None);
    assert_eq!(game.native_light([1, 400, 1]), (0, 0));
    // The overworld, parked.
    assert_eq!(
        game.native_level_block_collision(overworld, at),
        Some(vec![[0.0, 0.0, 0.0, 1.0, 1.0, 1.0]])
    );
    assert_eq!(
        game.native_level_biome(overworld, at).as_deref(),
        Some("minecraft:desert")
    );
    // Full sky light above the overworld's column.
    assert_eq!(
        game.native_level_light(overworld, [1, 400, 1]),
        Some((0, 15))
    );
    // The handle of the level in place, whose chunk is not loaded, and a block outside the world.
    assert_eq!(game.native_level_block_collision(nether, at), None);
    assert_eq!(game.native_level_biome(nether, at), None);
    assert_eq!(game.native_level_light(nether, at), None);
    assert_eq!(
        game.native_level_block_collision(overworld, [1, 5000, 1]),
        None
    );
}
