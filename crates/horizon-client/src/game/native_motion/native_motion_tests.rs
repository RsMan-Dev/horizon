//! `native_player_set_velocity` sends the player's client a velocity, and the anticheat's
//! movement replay hears it as it hears the server's own pushes.
use crate::game::{Game, lan::LanSettings};
use horizon_client::lan::{
    Joined, NetConfig, Out, PlayerSender, guest::Guest, hub::HubConfig, prediction::record::Change,
};
use horizon_protocol::{
    buf::{LpVec3, Reader},
    packets::{configuration::ClientInformation, play::Clientbound as C},
};
use std::sync::{Arc, atomic::AtomicI32, mpsc};

const PLAYER: u128 = 7;
const ENTITY: i32 = 41;

/// A server with one placed player whose movement is recorded, and what that player's client is sent.
fn fixture() -> (Game<'static>, mpsc::Receiver<Out>) {
    let mut game = crate::game::native_machines::tests::server_game();
    game.build_registries();
    game.lan_host(
        LanSettings {
            port: 0,
            guest_commands: false,
            force_game_mode: false,
        },
        NetConfig {
            announce: false,
            ..NetConfig::loopback(0, "set-velocity-test".into(), 1)
        },
        HubConfig {
            default_game_mode: 0,
            guest_commands: false,
            force_game_mode: false,
            hardcore: false,
            view_distance: 2,
            simulation_distance: 2,
            dimension: "minecraft:overworld".into(),
            allow_flight: false,
            seed: 0,
            max_players: 1,
            integrated: false,
            levels: Default::default(),
        },
    )
    .unwrap();
    let (tx, rx) = mpsc::channel();
    let mut guest = Guest::new(
        Joined {
            conn: 3,
            uuid: PLAYER,
            name: "Dasher".into(),
            properties: Vec::new(),
            info: ClientInformation::new(2),
            brand: "test".into(),
            sender: PlayerSender::new(tx, Arc::default()),
            latency: Arc::new(AtomicI32::new(0)),
            address: "127.0.0.1:25565".parse().unwrap(),
            expected: false,
            configured: Default::default(),
        },
        ENTITY,
        [0.5, 80.0, 0.5],
        2,
    );
    guest.placed = true;
    guest.record = Some(Box::default());
    game.lan.as_mut().unwrap().hub.guests.push(guest);
    (game, rx)
}

fn sent(rx: &mpsc::Receiver<Out>) -> Vec<C> {
    rx.try_iter()
        .flat_map(|out| match out {
            Out::Packet(body) => vec![body],
            Out::Packets(bodies) => bodies,
            Out::Disconnect(reason) => panic!("unexpected disconnect: {reason:?}"),
        })
        .map(|body| {
            let mut reader = Reader::new(&body);
            let id = reader.varint().unwrap();
            C::decode(id, reader.rest()).unwrap()
        })
        .collect()
}

#[test]
#[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
fn the_player_is_sent_the_velocity_and_the_replay_hears_it() {
    let (mut game, rx) = fixture();
    let launch = [0.25, 1.5, -0.5];
    assert!(game.native_player_set_velocity(PLAYER, launch));
    let packets = sent(&rx);
    let [C::SetEntityMotion(motion)] = packets.as_slice() else {
        panic!(
            "expected one SetEntityMotion, got {} packets",
            packets.len()
        );
    };
    assert_eq!(motion.id, ENTITY);
    assert_eq!(motion.velocity, LpVec3::quantize(launch));
    // The movement check plays the client's ticks again: it knows its client was pushed.
    let guest = game.lan.as_ref().unwrap().hub.guest(3).unwrap();
    assert_eq!(
        *guest.changes.lock().unwrap(),
        [Change::Velocity(LpVec3::quantize(launch))]
    );
}

#[test]
#[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
fn nothing_is_sent_for_a_player_not_here_or_a_value_that_is_not_finite() {
    let (mut game, rx) = fixture();
    assert!(!game.native_player_set_velocity(PLAYER + 1, [0.0, 1.0, 0.0]));
    for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert!(!game.native_player_set_velocity(PLAYER, [0.0, bad, 0.0]));
    }
    assert!(sent(&rx).is_empty());
}

#[test]
#[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
fn in_the_players_own_hook_the_packet_goes_after_the_hook() {
    let (mut game, rx) = fixture();
    // The hub is out of the game while the player's action runs.
    let hub = game.lan.take();
    game.lan_acting_uuid = Some(PLAYER);
    game.lan_acting_entity = Some(ENTITY as u32);
    assert!(game.native_player_set_velocity(PLAYER, [0.0, 1.0, 0.0]));
    assert!(sent(&rx).is_empty());
    let [C::SetEntityMotion(motion)] = game.guest_packets.as_slice() else {
        panic!("expected one SetEntityMotion after the hook");
    };
    assert_eq!(motion.id, ENTITY);
    game.lan = hub;
}
