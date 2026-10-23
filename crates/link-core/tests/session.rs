use link_core::session::SessionOffer;

#[test]
fn both_peers_derive_the_same_session_key() {
    let a = SessionOffer::generate(); let b = SessionOffer::generate();
    assert_eq!(a.derive_key(b.public_bytes(), b"desklink input v1"), b.derive_key(a.public_bytes(), b"desklink input v1"));
}
