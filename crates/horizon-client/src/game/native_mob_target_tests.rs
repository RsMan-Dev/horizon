//! `native_mob_set_target`: a mob given a target by a mod goes for it, and the refusals.
use super::super::*;
use super::lookup_tests::game;
use horizon_client::{chunk::Chunk, mobs::Who};
use horizon_core::position::ChunkPos;
use std::sync::Arc;

const DIM: &str = "minecraft:overworld";

/// A frozen level with a stone floor, in which `summon` puts mobs by UUID `[I;24723,0,0,n]`.
fn pen() -> Game<'static> {
    let mut game = game();
    game.world
        .chunks
        .insert((0, 0), Arc::new(Chunk::empty(0, 0)));
    game.block_ticks
        .add_empty_chunk(ChunkPos::new(0, 0))
        .unwrap();
    game.load_ticking_neighbourhood(0, 0);
    for command in [
        "difficulty normal",
        "forceload add 0 0 15 15",
        "fill 0 99 0 15 99 15 stone",
        "tick freeze",
    ] {
        assert!(game.run_command(command), "{command}");
    }
    game
}

/// Summons `kind` with UUID number `n` and its entity id.
fn summon(game: &mut Game<'_>, kind: &str, x: f64, n: u32, nbt: &str) -> u32 {
    let command = format!(
        "summon {kind} {x} 100 8.5 {{PersistenceRequired:1b,UUID:[I;24723,0,0,{n}],{nbt}}}"
    );
    assert!(game.run_command(&command), "{command}");
    let (_, id, _) = game
        .native_entity_by_uuid((24723u128 << 96) | u128::from(n))
        .expect("summoned");
    id
}

fn health(game: &Game<'_>, id: u32) -> f32 {
    game.mobs.iter().find(|m| m.id == id).unwrap().health
}

fn target(game: &Game<'_>, id: u32) -> Option<Who> {
    game.mobs.iter().find(|m| m.id == id).unwrap().ai.target
}

fn step(game: &mut Game<'_>, ticks: u32) {
    for _ in 0..ticks {
        assert!(game.run_command("tick step 1"));
        game.tick_server(false).unwrap();
    }
}

#[test]
#[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
fn a_mob_given_a_target_goes_for_it_until_it_is_told_to_let_go() {
    let mut game = pen();
    let sturdy =
        "NoAI:1b,Silent:1b,Health:100f,attributes:[{id:\"minecraft:max_health\",base:100d}]";
    let husk = summon(&mut game, "husk", 6.5, 1, "IsImmuneToZombification:1b");
    let pig = summon(&mut game, "pig", 7.5, 2, sturdy);
    // A husk has no reason to attack a pig on its own.
    step(&mut game, 60);
    assert_eq!(health(&game, pig), 100.0);
    assert!(game.native_mob_set_target(DIM, husk, Some(pig)));
    assert_eq!(target(&game, husk), Some(Who::Mob(pig)));
    step(&mut game, 60);
    assert!(health(&game, pig) < 100.0, "the husk went for the pig");
    // None lets go.
    assert!(game.native_mob_set_target(DIM, husk, None));
    assert_eq!(target(&game, husk), None);
}

#[test]
#[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
fn a_target_the_mob_cannot_have_is_refused_and_nothing_changes() {
    let mut game = pen();
    let husk = summon(&mut game, "husk", 6.5, 1, "NoAI:1b");
    let pig = summon(&mut game, "pig", 7.5, 2, "NoAI:1b");
    let ghast = summon(&mut game, "ghast", 9.5, 3, "NoAI:1b");
    let warden = summon(&mut game, "warden", 11.5, 4, "NoAI:1b");
    // The mob itself, a ghast (`Mob.canAttack`), an entity that is none and a level that is not.
    assert!(!game.native_mob_set_target(DIM, husk, Some(husk)));
    assert!(!game.native_mob_set_target(DIM, husk, Some(ghast)));
    assert!(!game.native_mob_set_target(DIM, husk, Some(pig + 1000)));
    assert!(!game.native_mob_set_target("minecraft:nowhere", husk, Some(pig)));
    assert!(!game.native_mob_set_target(DIM, pig + 1000, Some(pig)));
    assert_eq!(target(&game, husk), None);
    // A warden's brain holds its target.
    assert!(!game.native_mob_set_target(DIM, warden, Some(pig)));
    assert_eq!(target(&game, warden), None);
    // What is allowed still is.
    assert!(game.native_mob_set_target(DIM, husk, Some(pig)));
}
