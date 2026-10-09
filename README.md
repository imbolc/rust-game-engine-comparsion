# Rust game engine comparison

In this repo we compare Rust 2D game engines. They must run on mobile and web /
telegram mini-app.

To compare them we implement the same Tetris game using each of them. The game
is meant to look and behave exactly the same on every engine.

## Game engines

- <https://github.com/bevyengine/bevy> ([Tetris implementation](tetris-bevy/README.md))
- <https://github.com/FyroxEngine/Fyrox>
- <https://github.com/not-fl3/macroquad>

Each game should be implemented as it's own crate e.g. `tetris-bevy` without a
shared workspace and contain a readme with `cargo` command to run it.

## The Tetris game

Implement a minimal playable Tetris game.

Requirements:

- 10×20 grid with the 7 standard tetrominoes.
- Pieces fall automatically; arrow keys move and rotate them.
- Space drops a piece instantly.
- Completed rows disappear, with a sound, and the score increases.
- Game ends when a new piece cannot spawn.
- Display the board, score, and a restart button.
- Keep the implementation simple, with minimal code, dependencies, and UI.
