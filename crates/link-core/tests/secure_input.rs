use link_core::{InputEvent, InputOpener, InputSealer};
use link_core::protocol::InputPacket;

#[test]
fn encrypted_input_round_trips_and_rejects_replay() {
    let key = [7; 32];
    let mut sender = InputSealer::new(key, 100);
    let mut receiver = InputOpener::new(key);
    let wire = sender.seal(&InputPacket { sequence: 42, event: InputEvent::Key { scan_code: 30, pressed: true } }).unwrap();
    assert_eq!(receiver.open(&wire).unwrap().sequence, 42);
    assert!(receiver.open(&wire).is_err());
}

#[test]
fn tampered_input_is_rejected() {
    let key = [9; 32];
    let mut sender = InputSealer::new(key, 1);
    let mut wire = sender.seal(&InputPacket { sequence: 1, event: InputEvent::Scroll { horizontal: 0, vertical: 1 } }).unwrap();
    *wire.last_mut().unwrap() ^= 1;
    assert!(InputOpener::new(key).open(&wire).is_err());
}
