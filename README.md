# DeskLink

One Rust codebase, one application, two simultaneous roles: each device can send its screen and receive the other device's screen/input. The `dist/windows` and `dist/macos` folders are the release destinations for the standalone installers.

## What is implemented now

- Shared typed control and high-priority input packet protocol.
- Six-digit PIN challenge/proof primitive (to be used inside a Noise/QUIC session).
- Serializable multi-machine display layout and edge-crossing input routing.
- A minimal shared UI shell for pairing, layout, and stream quality.
- An authenticated UDP input wire format: ChaCha20-Poly1305 encryption and replay rejection. The desktop app has a strict no-plaintext UDP sender/receiver.
- A live TCP pairing exchange using SPAKE2 plus ephemeral X25519, with a 120-second pairing code and in-memory directional session keys.
- A local JSON trusted-peer record saved only after successful proof confirmation.
- Paired session keys connect to encrypted UDP input on port `49153`; the UI can send a harmless test pointer-motion packet and display authenticated receipt.
- Explicit platform traits for native screen capture and input injection.

The routing unit test captures the first build-order milestone: moving past a touching screen edge moves input ownership to the adjacent machine.

## Architecture boundaries

`crates/link-core` is OS-neutral and must not acquire platform APIs. `apps/desklink` owns networking and platform adapters:

- Windows: DXGI Desktop Duplication, `SendInput`, Raw Input/low-level hooks, FFmpeg with NVENC/QSV/AMF and D3D11VA.
- macOS: ScreenCaptureKit, `CGEventTap`/`CGEventPost`, FFmpeg VideoToolbox.
- Shared: QUIC + Noise-style authenticated pairing, mDNS discovery, UDP media/input paths, optional Opus.

Do not ship macOS virtual-display support based on private `CGVirtualDisplay` APIs. Treat it as an experimental development-only spike; ship physical-display/input sharing first. Windows virtual display requires a separately signed IddCx driver.

## Build prerequisites

Install Rust stable, Node 20+, the Tauri v2 prerequisites, and platform FFmpeg development libraries. Build releases on their native OS for proper codesigning:

```sh
cargo test -p link-core
cargo run -p desklink
```

The present repository is still not a remotely controllable desktop product: system-wide input capture/injection and screen capture/FFmpeg transport remain to be implemented.

## Security baseline before network testing

Never expose the control service to the public internet. Bind LAN interfaces only, authenticate every control and input packet after pairing, rate-limit pairing attempts, require explicit approval on the receiving device, and add an always-visible stop-control hotkey.
