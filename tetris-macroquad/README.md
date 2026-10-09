# Tetris with Macroquad

A standalone implementation of the [game requirements](../README.md), using
[Macroquad 0.4.16](https://docs.rs/macroquad/0.4.16/macroquad/). It uses the same
game rules, Fira Mono font, colors, layout, controls, and line-clear tone as the
Bevy version. The board scales to fit desktop and portrait mobile screens.

## Run locally

Requires Rust and a graphical desktop. Linux needs X11, OpenGL, and ALSA. On
Debian/Ubuntu:

```sh
sudo apt install libx11-6 libxi6 libgl1 libasound2-dev libasound2-plugins
```

On WSLg, follow the root [ALSA default-device setup](../README.md#linux-audio)
to avoid an audio-thread failure when no hardware sound card is available.

From this directory:

```sh
cargo run
```

From the repository root:

```sh
cargo run --manifest-path tetris-macroquad/Cargo.toml
```

## Controls

| Keyboard | Action |
| --- | --- |
| Left / Right | Move horizontally |
| Up | Rotate clockwise |
| Down | Drop instantly |
| R | Restart |

The on-screen buttons support mouse and touch, including held horizontal
movement. Pieces fall every 0.6 seconds; held movement repeats every 0.12
seconds. Rotation uses basic horizontal wall kicks. Every shuffled bag contains
all seven pieces. Each cleared row earns 100 points and plays an 880 Hz tone
for 120 milliseconds. A blocked spawn ends the game; restart is always available.

## Web, mobile browsers, and Telegram

Install the WebAssembly target once:

```sh
rustup target add wasm32-unknown-unknown
```

From this directory, build and serve:

```sh
cargo build --release --target wasm32-unknown-unknown
mkdir -p dist
cp index.html mq_js_bundle.js dist/
cp target/wasm32-unknown-unknown/release/tetris-macroquad.wasm dist/
python3 -m http.server 8080 --bind 0.0.0.0 --directory dist
```

Open `http://localhost:8080`, or the computer's LAN address on a phone. For
hosting, serve `dist/` over HTTPS and use its URL for a Telegram Mini App.
The font and tone are embedded in the WebAssembly binary. The local JavaScript
loader is the minified bundle shipped with Macroquad 0.4.16, including its
audio plugin, with a missing local declaration repaired in its networking
plugin. It does not depend on a CDN. Browsers enable audio after a tap or
keypress. The mobile path is this touch-enabled web build; native Android/iOS
packaging is not included.

The release profile uses `opt-level = "z"`, full LTO, one code-generation unit,
abort-on-panic, and stripped symbols. The root README documents the additional
WebAssembly optimization used for size comparisons.

## Check

```sh
cargo test -q
cargo check -q --target wasm32-unknown-unknown
cargo clippy -q --all-targets -- -D warnings
```

The ten headless tests cover the seven-piece bag, collisions, wall kicks,
locking, row compaction, scoring, game over, input timing, restart, and the
line-clear playback trigger.

## Bundled assets

`assets/FiraMono-subset.ttf` is Bevy 0.19.1's embedded default font, under the
SIL Open Font License in `assets/FiraMono-LICENSE`. `assets/clear.wav` is a
generated mono PCM sine wave. `mq_js_bundle.js` comes from the Macroquad 0.4.16
crate; its MIT and Apache 2.0 licenses are included in `assets/`.
