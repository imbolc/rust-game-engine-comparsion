# Tetris with Bevy

A standalone implementation of the [game requirements](../README.md), using
Bevy 0.19. The board and controls scale to fit the window, including portrait
mobile screens. Each shuffled bag contains all seven tetrominoes. Clearing a
row earns 100 points and plays a short tone. A blocked spawn ends the game.

## Run locally

Requires Rust 1.95 or newer and a graphical desktop. On Debian/Ubuntu, install
the native development and X11 runtime libraries first:

```sh
sudo apt install pkg-config libasound2-dev libasound2-plugins libx11-dev libxkbcommon-dev libxkbcommon-x11-0
```

From this directory:

```sh
cargo run
```

From the repository root:

```sh
cargo run --manifest-path tetris-bevy/Cargo.toml
```

## Controls

| Keyboard | Action |
| --- | --- |
| Left / Right | Move horizontally |
| Up | Rotate clockwise |
| Space | Drop instantly |
| R | Restart |

The on-screen buttons support mouse and touch. Hold a movement button or key
to repeat it. Rotation uses basic horizontal wall kicks. Restart is available
during play and after game over.

## Web, mobile browsers, and Telegram

Install [Trunk](https://trunk-rs.github.io/trunk/) and the WebAssembly target:

```sh
rustup target add wasm32-unknown-unknown
cargo install --locked trunk
```

From this directory, start the browser version:

```sh
trunk serve --release --address 0.0.0.0
```

Open `http://localhost:8080`, or the computer's LAN address on a phone. For
hosting:

```sh
trunk build --release
```

Serve the generated `dist/` directory over HTTPS and use that URL for a Telegram
Mini App. All visuals, fonts, and the generated line-clear sound are contained
in the build. Browsers may require a tap or keypress before allowing audio.
The mobile path is this touch-enabled web build; native Android/iOS packaging
is not included.

## Check

```sh
cargo test -q
cargo check -q --target wasm32-unknown-unknown
```

The game logic tests cover the seven-piece bag, collisions, rotation, locking,
row clearing, scoring, and game over. Headless Bevy tests exercise input,
gravity, score display, restart, and sound spawning without a graphics or audio
device.

To check desktop audio separately, run the optional playback test. It plays
the line-clear tone and verifies that Bevy opened an audio device:

```sh
cargo test -q line_clear_tone_reaches_the_audio_device -- --ignored
```

Sound plays when a completed row disappears. On PulseAudio desktops, including
WSLg, `libasound2-plugins` connects Bevy's ALSA output to the desktop audio server.
