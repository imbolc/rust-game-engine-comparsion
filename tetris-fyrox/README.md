# Tetris with Fyrox

A standalone implementation using [Fyrox 1.0.1](https://fyrox.rs/). It matches
the Bevy version's board, bundled Fira Mono font, colors, controls, seven-piece
bag, wall kicks, 100 points per cleared row, and 880 Hz line-clear tone. The
board and controls scale to fit the window, including portrait mobile screens.
Rendering, text, buttons, input, and audio use Fyrox. No editor is required.

## Run locally

Requires a current stable Rust toolchain, a graphical desktop with OpenGL 3.3,
and an audio output device. On Debian/Ubuntu:

```sh
sudo apt install pkg-config libasound2-dev libasound2-plugins libx11-dev libxkbcommon-dev libxkbcommon-x11-0
cargo run --manifest-path tetris-fyrox/Cargo.toml
```

From this directory, use `cargo run`.

On WSLg, follow the root [ALSA default-device setup](../README.md#linux-audio)
so the engine can open PulseAudio through ALSA.

On WSL, the game uses WSLg's X11 backend when `DISPLAY` is available to avoid
a Wayland compositor crash. Other desktops use the engine's automatic backend
selection.

## Controls

| Keyboard | Action |
| --- | --- |
| Left / Right | Move horizontally |
| Up | Rotate clockwise |
| Down | Drop instantly |
| R | Restart |

The on-screen buttons support mouse and touch. Hold a movement button or key
to repeat it every 120 ms. Pieces fall every 600 ms. Restart works during play
and after a blocked spawn ends the game. Sound plays when a completed row
disappears.

## Web, mobile browsers, and Telegram

Fyrox requires WebGL 2. Install the WebAssembly target and `wasm-bindgen-cli`
matching the `wasm-bindgen` version in `Cargo.lock`:

```sh
rustup target add wasm32-unknown-unknown
cargo install --locked wasm-bindgen-cli --version 0.2.129
```

From this directory:

```sh
cargo build -q --release --target wasm32-unknown-unknown --lib
wasm-bindgen --target web --out-dir pkg --no-typescript target/wasm32-unknown-unknown/release/tetris_fyrox.wasm
python3 -m http.server 8080 --bind 0.0.0.0
```

Open `http://localhost:8080`, or the computer's LAN address on a phone. Publish
`index.html` and `pkg/` together over HTTPS for a Telegram Mini App. Fonts and
the generated tone are embedded, so no asset folder needs to be served.
Browsers enable audio after a tap or keypress. The HTML entry resumes Fyrox's
audio context on those gestures and fits its canvas to the viewport.

The root [measurement script](../scripts/measure_wasm.py) also creates a
minified browser build. Mobile support uses this touch-enabled web build;
native Android/iOS packaging is not included.

## Check

```sh
cargo test -q
cargo clippy -q --all-targets -- -D warnings
cargo build -q --target wasm32-unknown-unknown --lib
```

Nine headless tests cover bags, collisions, drops, rotation, row ordering,
scoring, game over, movement repetition, restart timing, and complete plugin
initialization and update with an empty engine UI container.

Fyrox's published engine crate has no feature switches for removing its 3D
renderer or physics dependencies. This version uses its UI renderer directly
without creating a camera, meshes, or physics bodies.
