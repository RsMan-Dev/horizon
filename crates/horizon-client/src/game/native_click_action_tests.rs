//! A player's answer to a dialog button with a `custom` action reaches the mods as
//! `native_custom_click_action`, with the player's name and the payload the client sent.
use crate::game::lan::LanSettings;
use crate::mod_api::{CustomClickActionEvent, Hooks, Host};
use horizon_client::lan::{Joined, NetConfig, PlayerSender, guest::Guest, hub::HubConfig};
use horizon_protocol::packets::{
    configuration::{ClientInformation, CustomClickAction},
    play::Serverbound as S,
};
use horizon_world::nbt::{Compound, Tag};
use std::sync::{Arc, Mutex, atomic::AtomicI32, mpsc};

const PLAYER: u128 = 7;
type Heard = (u128, String, String, Option<Tag>);
static HEARD: Mutex<Vec<Heard>> = Mutex::new(Vec::new());

struct Listener;
impl Hooks for Listener {
    fn custom_click_action(&self, _: &mut dyn Host, event: &mut CustomClickActionEvent<'_>) {
        HEARD.lock().unwrap().push((
            event.player,
            event.name.to_owned(),
            event.id.to_owned(),
            event.payload.cloned(),
        ));
    }
}

#[test]
#[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
fn a_custom_click_action_is_heard_with_its_id_and_payload() {
    crate::mod_api::in_own_process(
        "game::native_hooks::native_click_action_tests::a_custom_click_action_is_heard_with_its_id_and_payload",
        heard_in_own_process,
    );
}

fn heard_in_own_process() {
    let mods: Vec<Box<dyn Hooks>> = vec![Box::new(Listener)];
    assert!(crate::mod_api::MODS.set(mods.into()).is_ok());
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
            ..NetConfig::loopback(0, "click-action-test".into(), 1)
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
    let (tx, _rx) = mpsc::channel();
    let mut guest = Guest::new(
        Joined {
            conn: 3,
            uuid: PLAYER,
            name: "Shopper".into(),
            properties: Vec::new(),
            info: ClientInformation::new(2),
            brand: "test".into(),
            sender: PlayerSender::new(tx, Arc::default()),
            latency: Arc::new(AtomicI32::new(0)),
            address: "127.0.0.1:25565".parse().unwrap(),
            expected: false,
            configured: Default::default(),
        },
        41,
        [0.5, 80.0, 0.5],
        2,
    );
    guest.placed = true;
    guest.loaded = true;
    game.lan.as_mut().unwrap().hub.guests.push(guest);
    let payload = Tag::Compound(Compound::from_iter([("item".into(), Tag::Int(3))]));
    for payload in [Some(payload), None] {
        let packet = S::CustomClickAction(CustomClickAction {
            id: "mymod:buy".into(),
            payload,
        });
        let bytes = packet.encode();
        let guest = game.lan.as_mut().unwrap().hub.guest_mut(3).unwrap();
        guest.inbox.push_back((packet, bytes.len()));
        guest.inbox_bytes += bytes.len();
        game.level_pass.first = false;
        game.level_pass.packets_drained = false;
        game.tick_lan_packets();
    }
    let heard = HEARD.lock().unwrap();
    let sent = |payload: Option<Tag>| {
        (
            PLAYER,
            "Shopper".to_owned(),
            "mymod:buy".to_owned(),
            payload,
        )
    };
    assert_eq!(heard.len(), 2);
    assert_eq!(
        heard[0],
        sent(Some(Tag::Compound(Compound::from_iter([(
            "item".into(),
            Tag::Int(3)
        )]))))
    );
    assert_eq!(heard[1], sent(None));
}
