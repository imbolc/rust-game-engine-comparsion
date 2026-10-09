mod game;

use game::{Game, HEIGHT, WIDTH};
use macroquad::{audio, prelude::*};

const COLORS: [Color; 8] = [
    Color::new(0.10, 0.12, 0.17, 1.0),
    Color::new(0.20, 0.80, 0.90, 1.0),
    Color::new(0.95, 0.80, 0.20, 1.0),
    Color::new(0.65, 0.35, 0.85, 1.0),
    Color::new(0.30, 0.80, 0.40, 1.0),
    Color::new(0.90, 0.30, 0.35, 1.0),
    Color::new(0.25, 0.45, 0.90, 1.0),
    Color::new(0.95, 0.55, 0.20, 1.0),
];

#[derive(Clone, Copy)]
enum Action {
    Left,
    Rotate,
    Right,
    Drop,
    Restart,
}

const KEYS: [(KeyCode, Action); 5] = [
    (KeyCode::Left, Action::Left),
    (KeyCode::Right, Action::Right),
    (KeyCode::Up, Action::Rotate),
    (KeyCode::Down, Action::Drop),
    (KeyCode::R, Action::Restart),
];

const BUTTONS: [(&str, Action, Rect, u16); 5] = [
    ("<", Action::Left, Rect::new(45.0, 647.6, 58.0, 44.0), 15),
    (
        "Rotate",
        Action::Rotate,
        Rect::new(109.0, 647.6, 58.0, 44.0),
        15,
    ),
    (">", Action::Right, Rect::new(173.0, 647.6, 58.0, 44.0), 15),
    (
        "Drop",
        Action::Drop,
        Rect::new(237.0, 647.6, 58.0, 44.0),
        15,
    ),
    (
        "Restart",
        Action::Restart,
        Rect::new(115.0, 701.6, 110.0, 44.0),
        18,
    ),
];

struct Session {
    game: Game,
    gravity: f32,
    repeat: f32,
}

impl Session {
    fn new(seed: u64) -> Self {
        Self {
            game: Game::new(seed),
            gravity: 0.0,
            repeat: 0.0,
        }
    }

    fn update(&mut self, dt: f32, mut actions: Vec<Action>, held: &[Action], seed: u64) -> u32 {
        if held.is_empty() {
            self.repeat = 0.0;
        } else {
            self.repeat += dt;
            if self.repeat >= 0.12 {
                self.repeat %= 0.12;
                actions.extend_from_slice(held);
            }
        }
        let mut cleared = 0;
        for action in actions {
            match action {
                Action::Left => {
                    self.game.move_piece(-1);
                }
                Action::Right => {
                    self.game.move_piece(1);
                }
                Action::Rotate => {
                    self.game.rotate();
                }
                Action::Drop => {
                    cleared += self.game.drop();
                    self.gravity = 0.0;
                }
                Action::Restart => *self = Self::new(seed),
            }
        }
        if !self.game.over {
            self.gravity += dt;
            if self.gravity >= 0.6 {
                self.gravity %= 0.6;
                cleared += self.game.step();
            }
        }
        cleared
    }
}

fn window_conf() -> Conf {
    Conf {
        window_title: "Tetris - Macroquad".into(),
        window_width: 400,
        window_height: 800,
        high_dpi: true,
        icon: None,
        ..Default::default()
    }
}

fn seed() -> u64 {
    (miniquad::date::now() * 1_000_000_000.0) as u64
}

#[macroquad::main(window_conf)]
async fn main() {
    let font = load_ttf_font_from_bytes(include_bytes!("../assets/FiraMono-subset.ttf"))
        .expect("embedded Fira Mono font must be valid");
    let sound = audio::load_sound_from_bytes(include_bytes!("../assets/clear.wav"))
        .await
        .expect("embedded line-clear tone must be valid");
    simulate_mouse_with_touch(false);
    let mut session = Session::new(seed());
    let mut previous_pressed = [false; 5];
    loop {
        let scale = (screen_width() / 360.0)
            .min(screen_height() / 800.0)
            .max(0.1);
        let origin = vec2(
            (screen_width() - 340.0 * scale) / 2.0,
            (screen_height() - 780.0 * scale) / 2.0,
        );
        let mut actions = Vec::new();
        let mut held = Vec::new();
        for (key, action) in KEYS {
            if is_key_pressed(key) {
                actions.push(action);
            } else if is_key_down(key) && matches!(action, Action::Left | Action::Right) {
                held.push(action);
            }
        }
        let pointer = (Vec2::from(mouse_position()) - origin) / scale;
        // Touch positions use physical pixels; mouse_position() already accounts for DPI.
        let touches = touches();
        let mut pressed = [false; 5];
        for (i, (_, action, rect, _)) in BUTTONS.iter().enumerate() {
            pressed[i] = (rect.contains(pointer) && is_mouse_button_down(MouseButton::Left))
                || touches.iter().any(|touch| {
                    !matches!(touch.phase, TouchPhase::Ended | TouchPhase::Cancelled)
                        && rect.contains((touch.position / screen_dpi_scale() - origin) / scale)
                });
            if pressed[i] {
                if !previous_pressed[i] {
                    actions.push(*action);
                } else if matches!(action, Action::Left | Action::Right) {
                    held.push(*action);
                }
            }
        }
        previous_pressed = pressed;
        if session.update(get_frame_time().min(0.25), actions, &held, seed()) > 0 {
            audio::play_sound_once(&sound);
        }
        clear_background(Color::new(0.05, 0.06, 0.09, 1.0));
        label(
            "Tetris",
            28,
            Rect::new(0.0, 0.0, 340.0, 33.6),
            &font,
            origin,
            scale,
        );
        let score = if session.game.over {
            format!("Game over | Score: {}", session.game.score)
        } else {
            format!("Score: {}", session.game.score)
        };
        label(
            &score,
            20,
            Rect::new(0.0, 43.6, 340.0, 24.0),
            &font,
            origin,
            scale,
        );
        rectangle(
            Rect::new(30.0, 77.6, 280.0, 560.0),
            Color::new(0.22, 0.25, 0.32, 1.0),
            origin,
            scale,
        );
        let blocks = session.game.blocks();
        for y in 0..HEIGHT {
            for x in 0..WIDTH {
                let value = if !session.game.over && blocks.contains(&(x as i32, y as i32)) {
                    session.game.color()
                } else {
                    session.game.board[y][x]
                };
                rectangle(
                    Rect::new(31.0 + x as f32 * 28.0, 78.6 + y as f32 * 28.0, 26.0, 26.0),
                    COLORS[value as usize],
                    origin,
                    scale,
                );
            }
        }
        for (i, (text, _, rect, size)) in BUTTONS.iter().enumerate() {
            let color = if pressed[i] {
                Color::new(0.40, 0.48, 0.60, 1.0)
            } else if rect.contains(pointer) {
                Color::new(0.28, 0.34, 0.44, 1.0)
            } else {
                Color::new(0.20, 0.24, 0.32, 1.0)
            };
            rectangle(*rect, color, origin, scale);
            label(text, *size, *rect, &font, origin, scale);
        }
        label(
            "Left/Right: move | Up: rotate | Down: drop",
            12,
            Rect::new(0.0, 755.6, 340.0, 14.4),
            &font,
            origin,
            scale,
        );
        next_frame().await;
    }
}

fn rectangle(rect: Rect, color: Color, origin: Vec2, scale: f32) {
    draw_rectangle(
        origin.x + rect.x * scale,
        origin.y + rect.y * scale,
        rect.w * scale,
        rect.h * scale,
        color,
    );
}

fn label(text: &str, size: u16, rect: Rect, font: &Font, origin: Vec2, scale: f32) {
    let width = measure_text(text, Some(font), size, scale).width;
    // Fira Mono's ascent/descent are 0.935/0.265 em, matching Bevy's text baseline.
    draw_text_ex(
        text,
        origin.x + (rect.x + rect.w / 2.0) * scale - width / 2.0,
        origin.y + (rect.y + rect.h / 2.0 + 0.335 * size as f32) * scale,
        TextParams {
            font: Some(font),
            font_size: size,
            font_scale: scale,
            color: WHITE,
            ..Default::default()
        },
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gravity_and_held_movement_follow_their_intervals() {
        let mut session = Session::new(1);
        let before = session.game.blocks();
        session.update(0.1, vec![], &[Action::Left], 2);
        assert_eq!(session.game.blocks(), before);
        session.update(0.03, vec![], &[Action::Left], 2);
        assert_eq!(session.game.blocks(), before.map(|(x, y)| (x - 1, y)));
        session.update(0.48, vec![], &[], 2);
        assert_eq!(session.game.blocks(), before.map(|(x, y)| (x - 1, y + 1)));
    }

    #[test]
    fn drop_and_restart_reset_the_board_and_timers() {
        let mut session = Session::new(1);
        session.update(0.0, vec![Action::Drop], &[], 2);
        assert_eq!(
            session
                .game
                .board
                .iter()
                .flatten()
                .filter(|&&c| c != 0)
                .count(),
            4
        );
        session.game.score = 100;
        session.game.over = true;
        session.update(0.0, vec![Action::Restart], &[], 2);
        assert!(!session.game.over);
        assert_eq!(session.game.score, 0);
        assert_eq!(session.game.board, [[0; WIDTH]; HEIGHT]);
        assert_eq!((session.gravity, session.repeat), (0.0, 0.0));
    }

    #[test]
    fn a_row_clear_reports_sound_playback() {
        let mut session = Session::new(1);
        let blocks = session.game.blocks();
        let bottom = blocks.iter().map(|&(_, y)| y).max().unwrap();
        session.game.board[HEIGHT - 1].fill(1);
        for (x, y) in blocks {
            if y == bottom {
                session.game.board[HEIGHT - 1][x as usize] = 0;
            }
        }
        assert_eq!(session.update(0.0, vec![Action::Drop], &[], 2), 1);
        assert_eq!(session.game.score, 100);
    }
}
