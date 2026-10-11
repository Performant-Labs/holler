//! D14: `*/watch` is wired, with one minimal test each way. Against a real hub,
//! `holler pane watch --until-idle --format=json` from cursor 0 prints the put of a record
//! written through the client, each line a valid envelope; and `ProfileStore::watch` of the
//! client yields the put of a profile from cursor 0.

use holler_cli::pane::wiring::store::HubProfileStore;
use holler_pane::{Actor, Cursor, ProfileStore};
use holler_pane_testkit::envelope::check_envelope;
use holler_pane_testkit::fixture::sample_profile;

use crate::store_only::{Bare, PANE};
use crate::with_state_dir;

#[test]
fn pane_watch_prints_the_put_of_a_client_write_each_line_an_envelope() {
    let bare = Bare::start();
    bare.seed_pane(PANE);

    let out = bare.holler(&["pane", "watch", "--until-idle", "--format=json"]);
    assert_eq!(out.code, 0, "{out:?}");
    let mut puts = Vec::new();
    for line in out.stdout.lines() {
        let envelope = check_envelope(&format!("{line}\n"), 0)
            .unwrap_or_else(|fault| panic!("each watch line is an envelope: {fault}: {line}"));
        if envelope.data["change"] == "put" && envelope.data["name"] == PANE {
            puts.push(envelope.data);
        }
    }
    assert_eq!(puts.len(), 1, "one put of {PANE} from cursor 0: {out:?}");
    assert!(
        puts[0]["cursor"].as_u64().is_some_and(|c| c > 0),
        "the put carries its cursor: {:?}",
        puts[0]
    );
}

#[test]
fn the_profile_clients_watch_yields_a_put_from_cursor_0() {
    let bare = Bare::start();
    let profile = sample_profile("hlr649", &[]).unwrap();
    let actor = Actor::parse("holler pane").unwrap();
    let first = with_state_dir(bare.state.path(), || {
        let store = HubProfileStore::new();
        store.cas_put(&profile, 0, &actor).expect("cas_put through the client");
        let mut watch = store.watch(Cursor(0)).expect("watch through the client");
        watch.next()
    });
    let event = first
        .expect("the watch yields an item")
        .expect("the item is not an error")
        .expect("from cursor 0 the item is the current state, not idle");
    assert_eq!(event.name.as_str(), "hlr649");
    let stored = event.profile.expect("a put carries the record");
    assert_eq!(stored.generation, 1, "the stored record: {stored:?}");
}
