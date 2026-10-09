mod game;

use std::time::Duration;

use bevy::{audio::Pitch, prelude::*};
use game::{Game, HEIGHT, WIDTH};
use web_time::{SystemTime, UNIX_EPOCH};

const COLORS: [Color; 8] = [
    Color::srgb(0.10, 0.12, 0.17),
    Color::srgb(0.20, 0.80, 0.90),
    Color::srgb(0.95, 0.80, 0.20),
    Color::srgb(0.65, 0.35, 0.85),
    Color::srgb(0.30, 0.80, 0.40),
    Color::srgb(0.90, 0.30, 0.35),
    Color::srgb(0.25, 0.45, 0.90),
    Color::srgb(0.95, 0.55, 0.20),
];
const BUTTON: Color = Color::srgb(0.20, 0.24, 0.32);

#[derive(Resource)]
struct Session {
    game: Game,
    gravity: Timer,
    repeat: Timer,
}

impl Default for Session {
    fn default() -> Self {
        Self {
            game: Game::new(seed()),
            gravity: Timer::from_seconds(0.6, TimerMode::Repeating),
            repeat: Timer::from_seconds(0.12, TimerMode::Repeating),
        }
    }
}

#[derive(Resource)]
struct ClearSound(Handle<Pitch>);

#[derive(Component)]
struct Cell(i32, i32);

#[derive(Component)]
struct Score;

#[derive(Component, Clone, Copy)]
enum Action {
    Left,
    Right,
    Rotate,
    Drop,
    Restart,
}

const KEYS: [(KeyCode, Action); 5] = [
    (KeyCode::ArrowLeft, Action::Left),
    (KeyCode::ArrowRight, Action::Right),
    (KeyCode::ArrowUp, Action::Rotate),
    (KeyCode::ArrowDown, Action::Drop),
    (KeyCode::KeyR, Action::Restart),
];

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Tetris - Bevy".into(),
                resolution: (400, 800).into(),
                canvas: Some("#game".into()),
                fit_canvas_to_parent: true,
                prevent_default_event_handling: true,
                ..default()
            }),
            ..default()
        }))
        .insert_resource(ClearColor(Color::srgb(0.05, 0.06, 0.09)))
        .init_resource::<Session>()
        .add_systems(Startup, setup)
        .add_systems(
            PreUpdate,
            prioritize_touch
                .after(bevy::input::InputSystems)
                .before(bevy::ui::UiSystems::Focus),
        )
        .add_systems(Update, (resize, play, draw).chain())
        .run();
}

fn prioritize_touch(touches: Res<bevy::input::touch::Touches>, mut windows: Query<&mut Window>) {
    // UI focus otherwise prefers a stale mouse cursor over the active finger.
    if touches.first_pressed_position().is_some() {
        for mut window in &mut windows {
            window.set_cursor_position(None);
        }
    }
}

fn seed() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64
}

fn setup(mut commands: Commands, mut pitches: ResMut<Assets<Pitch>>) {
    commands.spawn(Camera2d);
    commands.insert_resource(ClearSound(
        pitches.add(Pitch::new(880.0, Duration::from_millis(120))),
    ));
    commands
        .spawn(Node {
            width: percent(100),
            height: percent(100),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            ..default()
        })
        .with_children(|root| {
            root.spawn(Node {
                width: px(340),
                height: px(780),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                row_gap: px(10),
                ..default()
            })
            .with_children(|ui| {
                ui.spawn((Text::new("Tetris"), font(28.0)));
                ui.spawn((Text::new("Score: 0"), font(20.0), Score));
                ui.spawn((
                    Node {
                        display: Display::Grid,
                        grid_template_columns: vec![RepeatedGridTrack::px(WIDTH as u16, 26.0)],
                        grid_template_rows: vec![RepeatedGridTrack::px(HEIGHT as u16, 26.0)],
                        column_gap: px(2),
                        row_gap: px(2),
                        padding: UiRect::all(px(1)),
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.22, 0.25, 0.32)),
                ))
                .with_children(|board| {
                    for y in 0..HEIGHT {
                        for x in 0..WIDTH {
                            board.spawn((
                                Node::default(),
                                BackgroundColor(COLORS[0]),
                                Cell(x as i32, y as i32),
                            ));
                        }
                    }
                });
                ui.spawn(Node {
                    column_gap: px(6),
                    ..default()
                })
                .with_children(|controls| {
                    for (label, action) in [
                        ("<", Action::Left),
                        ("Rotate", Action::Rotate),
                        (">", Action::Right),
                        ("Drop", Action::Drop),
                    ] {
                        controls
                            .spawn((Button, button_node(58.0), BackgroundColor(BUTTON), action))
                            .with_child((Text::new(label), font(15.0)));
                    }
                });
                ui.spawn((
                    Button,
                    button_node(110.0),
                    BackgroundColor(BUTTON),
                    Action::Restart,
                ))
                .with_child((Text::new("Restart"), font(18.0)));
                ui.spawn((
                    Text::new("Left/Right: move | Up: rotate | Down: drop"),
                    font(12.0),
                ));
            });
        });
}

fn font(size: f32) -> TextFont {
    TextFont {
        font_size: FontSize::Px(size),
        ..default()
    }
}

fn button_node(width: f32) -> Node {
    Node {
        width: px(width),
        height: px(44),
        align_items: AlignItems::Center,
        justify_content: JustifyContent::Center,
        ..default()
    }
}

fn resize(window: Single<&Window>, mut scale: ResMut<UiScale>) {
    scale.0 = (window.width() / 360.0)
        .min(window.height() / 800.0)
        .max(0.1);
}

fn play(
    mut commands: Commands,
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    mut session: ResMut<Session>,
    sound: Res<ClearSound>,
    mut buttons: Query<(&Action, Ref<Interaction>, &mut BackgroundColor), With<Button>>,
) {
    let mut actions = Vec::new();
    let mut held = Vec::new();
    for (key, action) in KEYS {
        if keys.just_pressed(key) {
            actions.push(action);
        } else if keys.pressed(key) && matches!(action, Action::Left | Action::Right) {
            held.push(action);
        }
    }
    for (&action, interaction, mut color) in &mut buttons {
        color.0 = match *interaction {
            Interaction::Pressed => Color::srgb(0.40, 0.48, 0.60),
            Interaction::Hovered => Color::srgb(0.28, 0.34, 0.44),
            Interaction::None => BUTTON,
        };
        if *interaction == Interaction::Pressed {
            if interaction.is_changed() {
                actions.push(action);
            } else if matches!(action, Action::Left | Action::Right) {
                held.push(action);
            }
        }
    }
    if held.is_empty() {
        session.repeat.reset();
    } else if session.repeat.tick(time.delta()).just_finished() {
        actions.extend(held);
    }
    let mut cleared = 0;
    for action in actions {
        match action {
            Action::Left => {
                session.game.move_piece(-1);
            }
            Action::Right => {
                session.game.move_piece(1);
            }
            Action::Rotate => {
                session.game.rotate();
            }
            Action::Drop => {
                cleared += session.game.drop();
                session.gravity.reset();
            }
            Action::Restart => {
                session.game = Game::new(seed());
                session.gravity.reset();
                session.repeat.reset();
            }
        }
    }
    if !session.game.over && session.gravity.tick(time.delta()).just_finished() {
        cleared += session.game.step();
    }
    if cleared > 0 {
        commands.spawn((AudioPlayer(sound.0.clone()), PlaybackSettings::DESPAWN));
    }
}

fn draw(
    session: Res<Session>,
    mut cells: Query<(&Cell, &mut BackgroundColor)>,
    mut score: Single<&mut Text, With<Score>>,
) {
    let game = &session.game;
    let blocks = game.blocks();
    for (&Cell(x, y), mut color) in &mut cells {
        let value = if !game.over && blocks.contains(&(x, y)) {
            game.color()
        } else {
            game.board[y as usize][x as usize]
        };
        color.0 = COLORS[value as usize];
    }
    let label = if game.over {
        format!("Game over | Score: {}", game.score)
    } else {
        format!("Score: {}", game.score)
    };
    if score.0 != label {
        score.0 = label;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::{audio::PlaybackMode, time::TimeUpdateStrategy};

    #[test]
    #[ignore = "requires a desktop audio device"]
    fn line_clear_tone_reaches_the_audio_device() {
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            AssetPlugin::default(),
            bevy::audio::AudioPlugin::default(),
        ))
        .add_systems(Startup, setup);
        app.update();
        let sound = app.world().resource::<ClearSound>().0.clone();
        let player = app
            .world_mut()
            .spawn((AudioPlayer(sound), PlaybackSettings::DESPAWN))
            .id();
        app.update();
        assert!(
            app.world().get::<AudioSink>(player).is_some(),
            "Bevy could not open an audio output device"
        );
        std::thread::sleep(Duration::from_millis(300));
    }

    fn headless_app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(
                210,
            )))
            .init_resource::<Assets<Pitch>>()
            .init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<UiScale>()
            .init_resource::<Session>()
            .add_systems(Startup, setup)
            .add_systems(Update, (resize, play, draw).chain());
        app.world_mut().spawn(Window {
            resolution: (360, 800).into(),
            ..default()
        });
        app.update();
        app
    }

    #[test]
    fn keyboard_drops_and_touch_restart_updates_the_board_and_score() {
        let mut app = headless_app();
        let world = app.world_mut();
        assert_eq!(world.query::<&Cell>().iter(world).count(), WIDTH * HEIGHT);
        world
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::ArrowDown);
        app.update();
        assert_eq!(
            app.world()
                .resource::<Session>()
                .game
                .board
                .iter()
                .flatten()
                .filter(|&&cell| cell != 0)
                .count(),
            4
        );

        let world = app.world_mut();
        world
            .resource_mut::<ButtonInput<KeyCode>>()
            .reset(KeyCode::ArrowDown);
        {
            let mut session = world.resource_mut::<Session>();
            session.game.over = true;
            session.game.score = 123;
        }
        app.update();
        let world = app.world_mut();
        assert_eq!(
            world
                .query_filtered::<&Text, With<Score>>()
                .single(world)
                .unwrap()
                .0,
            "Game over | Score: 123"
        );
        for (action, mut interaction) in
            world.query::<(&Action, &mut Interaction)>().iter_mut(world)
        {
            if matches!(action, Action::Restart) {
                *interaction = Interaction::Pressed;
            }
        }
        app.update();
        let world = app.world_mut();
        let game = &world.resource::<Session>().game;
        assert!(!game.over);
        assert_eq!(game.score, 0);
        assert_eq!(game.board, [[0; WIDTH]; HEIGHT]);
        assert_eq!(
            world
                .query_filtered::<&Text, With<Score>>()
                .single(world)
                .unwrap()
                .0,
            "Score: 0"
        );
    }

    #[test]
    fn touch_focus_restarts_with_a_stale_mouse_cursor_at_high_dpi() {
        use bevy::{
            app::HierarchyPropagatePlugin,
            input::{
                InputPlugin,
                touch::{TouchInput, TouchPhase},
            },
            ui::{
                ComputedUiTargetCamera, UiStack, ui_focus_system,
                update::propagate_ui_target_cameras,
            },
            window::PrimaryWindow,
        };

        let mut app = headless_app();
        app.add_plugins((
            InputPlugin,
            HierarchyPropagatePlugin::<ComputedUiTargetCamera>::new(PostUpdate),
        ))
        .init_resource::<UiStack>()
        .add_systems(PostUpdate, propagate_ui_target_cameras)
        .add_systems(
            PreUpdate,
            (prioritize_touch, ui_focus_system)
                .chain()
                .after(bevy::input::InputSystems),
        );
        let world = app.world_mut();
        let window = world
            .query_filtered::<Entity, With<Window>>()
            .single(world)
            .unwrap();
        world.entity_mut(window).insert(PrimaryWindow);
        {
            let mut window = world.get_mut::<Window>(window).unwrap();
            window.resolution = (400, 800).into();
            window
                .resolution
                .set_scale_factor_and_apply_to_physical_size(2.0);
            window.set_cursor_position(Some(Vec2::new(10.0, 10.0)));
        }
        let restart = world
            .query::<(Entity, &Action)>()
            .iter(world)
            .find_map(|(entity, action)| matches!(action, Action::Restart).then_some(entity))
            .unwrap();
        world.entity_mut(restart).insert((
            ComputedNode {
                size: Vec2::new(220.0, 88.0),
                ..default()
            },
            UiGlobalTransform::from_translation(Vec2::new(400.0, 1468.0)),
            InheritedVisibility::VISIBLE,
        ));
        world.insert_resource(UiStack {
            partition: std::iter::once(0..1).collect(),
            uinodes: vec![restart],
        });
        world.resource_mut::<Session>().game.drop();
        app.update();
        app.update();
        app.world_mut().write_message(TouchInput {
            phase: TouchPhase::Started,
            position: Vec2::new(200.0, 734.0),
            window,
            force: None,
            id: 1,
        });
        app.update();
        assert_eq!(
            app.world().resource::<Session>().game.board,
            [[0; WIDTH]; HEIGHT]
        );
    }

    #[test]
    fn elapsed_time_moves_the_active_piece_down() {
        let mut app = headless_app();
        let before = app.world().resource::<Session>().game.blocks();
        for _ in 0..3 {
            app.update();
        }
        assert_eq!(
            app.world().resource::<Session>().game.blocks(),
            before.map(|(x, y)| (x, y + 1))
        );
    }

    #[test]
    fn clearing_a_row_spawns_the_clear_sound() {
        let mut app = headless_app();
        {
            let mut session = app.world_mut().resource_mut::<Session>();
            let blocks = session.game.blocks();
            let bottom = blocks.iter().map(|&(_, y)| y).max().unwrap();
            session.game.board[HEIGHT - 1].fill(1);
            for (x, y) in blocks {
                if y == bottom {
                    session.game.board[HEIGHT - 1][x as usize] = 0;
                }
            }
        }
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::ArrowDown);
        app.update();
        let world = app.world_mut();
        assert_eq!(world.resource::<Session>().game.score, 100);
        let (player, settings) = world
            .query::<(&AudioPlayer<Pitch>, &PlaybackSettings)>()
            .single(world)
            .unwrap();
        assert_eq!(player.0, world.resource::<ClearSound>().0);
        assert!(matches!(settings.mode, PlaybackMode::Despawn));
    }
}
