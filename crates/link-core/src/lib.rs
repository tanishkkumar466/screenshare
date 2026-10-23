//! Platform-neutral protocol and routing primitives for DeskLink.
//! OS capture/injection implementations live behind traits in the desktop app.

pub mod input;
pub mod layout;
pub mod pairing;
pub mod protocol;
pub mod secure_input;
pub mod session;
pub mod pake;

pub use input::{InputEvent, InputRouter, PointerMotion, Route};
pub use layout::{Display, DisplayLayout, Edge, MachineId, Rect};
pub use secure_input::{InputOpener, InputSealer, SecureInputError};
