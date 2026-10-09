# Rust game engine comparison

In this repo we compare Rust 2D game engines. They must run on mobile and web /
telegram mini-app.

To compare them we implement the same Tetris game using each of them. The game
is meant to look and behave exactly the same on every engine.

## Game engines

- <https://github.com/bevyengine/bevy>
  ([Tetris implementation](tetris-bevy/README.md))
- <https://github.com/FyroxEngine/Fyrox>
  ([Tetris implementation](tetris-fyrox/README.md))
- <https://github.com/not-fl3/macroquad>
  ([Tetris implementation](tetris-macroquad/README.md))

Each game should be implemented as it's own crate e.g. `tetris-bevy` without a
shared workspace and contain a readme with `cargo` command to run it.

All three implementations run on desktop and in mobile browsers, with touch
controls and a layout that scales to the viewport. Their web builds can be
hosted over HTTPS for Telegram Mini Apps. Native Android/iOS packaging is not
included.

## Release WebAssembly size and LOC

The comparison includes each game's embedded font and line-clear sound. Sizes
cover the WebAssembly binary; HTML and JavaScript loaders are excluded.

Measured on 2026-10-09 with Rust 1.99.0, `wasm-bindgen` 0.2.129, and
Binaryen 120. All sizes are bytes.

| Engine           | Release WASM | Minified WASM |      Gzip | Rust LOC |
| ---------------- | -----------: | ------------: | --------: | -------: |
| Bevy 0.19.1      |   16,985,342 |    14,553,004 | 4,580,435 |      432 |
| Fyrox 1.0.1      |   15,878,670 |    13,139,529 | 5,142,453 |      628 |
| Macroquad 0.4.16 |      384,557 |       342,135 |   145,184 |      412 |

Measurements use the same release settings: `opt-level = "z"`, full LTO, one
code-generation unit, abort-on-panic, and stripped symbols. Bevy and Fyrox pass
through `wasm-bindgen` before measurement. All three binaries then pass through
Binaryen's `wasm-opt -Oz --strip-debug --strip-producers`; gzip uses level 9
with a reproducible header.

Rust LOC counts physical lines containing code in `src/**/*.rs` and `build.rs`,
excluding blank lines, comments, and test modules. Each standalone crate's
copied game logic is included; dependencies, generated bindings, and vendored
JavaScript are excluded.

To reproduce the comparison, install the WebAssembly target, Binaryen
(`sudo apt install binaryen` on Debian/Ubuntu), and the matching binding tool:

```sh
rustup target add wasm32-unknown-unknown
cargo install --locked wasm-bindgen-cli --version 0.2.129
python3 scripts/measure_wasm.py
```

The [measurement script](scripts/measure_wasm.py) builds all three games, prints
the table, and writes tool versions, exact optimization flags, byte counts, and
runnable browser bundles to `target/wasm-size/`. Run
`python3 -m http.server 8765 --directory target/wasm-size` and open `/bevy/`,
`/fyrox/`, or `/macroquad/` to play those measured builds.

## Linux audio

Fyrox and Macroquad open ALSA's `default` playback device. On WSLg, route that
device to PulseAudio. Installing `libasound2-plugins` adds a named `pulse`
device but does not select it as the default. Debian provides a configuration
example; enable it if the default device still points to a missing hardware
card and this configuration has not already been enabled:

```sh
sudo ln -s /etc/alsa/conf.d/99-pulseaudio-default.conf.example /etc/alsa/conf.d/99-pulseaudio-default.conf
```

The configuration uses the existing `PULSE_SERVER` setting. Fyrox selects X11
on WSLg when `DISPLAY` is available, avoiding the Wayland compositor failure
seen during native startup.

After building both native binaries, run the [native startup check](scripts/smoke_native.py):

```sh
python3 scripts/smoke_native.py
```

It opens the ALSA default playback device, launches each game for five seconds,
and saves complete startup logs to `target/native-smoke/`.

## The Tetris game

Implement a minimal playable Tetris game.

Requirements:

- 10×20 grid with the 7 standard tetrominoes.
- Pieces fall automatically; arrow keys move and rotate them.
- Down drops a piece instantly.
- Completed rows disappear, with a sound, and the score increases.
- Game ends when a new piece cannot spawn.
- Display the board, score, and a restart button.
- Keep the implementation simple, with minimal code, dependencies, and UI.
