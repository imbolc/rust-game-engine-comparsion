mod game;

use fyrox::{
    asset::{Resource, untyped::ResourceKind},
    core::{
        algebra::{Matrix3, Vector2},
        color::Color,
        pool::Handle,
        reflect::prelude::*,
        uuid::uuid,
        visitor::prelude::*,
    },
    dpi::LogicalSize,
    engine::{GraphicsContextParams, executor::Executor},
    event::{ElementState, Event, WindowEvent},
    event_loop::EventLoop,
    gui::{
        BuildContext, HorizontalAlignment, Thickness, UiNode, UserInterface, VerticalAlignment,
        border::BorderBuilder,
        brush::Brush,
        button::ButtonBuilder,
        canvas::CanvasBuilder,
        decorator::DecoratorBuilder,
        font::{Font, FontResource, FontStyles},
        message::{MessageDirection, MouseButton, UiMessage},
        text::{TextBuilder, TextMessage},
        widget::{WidgetBuilder, WidgetMessage},
    },
    keyboard::{KeyCode, PhysicalKey},
    plugin::{Plugin, PluginContext, error::GameResult},
    scene::{
        Scene,
        base::BaseBuilder,
        node::Node,
        sound::{DataSource, SoundBuffer, SoundBuilder},
    },
    window::WindowAttributes,
};
use game::{Game, HEIGHT, WIDTH};
use web_time::{SystemTime, UNIX_EPOCH};

const COLORS: [Color; 8] = [
    Color::opaque(26, 31, 43),
    Color::opaque(51, 204, 230),
    Color::opaque(242, 204, 51),
    Color::opaque(166, 89, 217),
    Color::opaque(77, 204, 102),
    Color::opaque(230, 77, 89),
    Color::opaque(64, 115, 230),
    Color::opaque(242, 140, 51),
];

#[derive(Clone, Copy, PartialEq)]
enum Action {
    Left,
    Rotate,
    Right,
    Drop,
    Restart,
}

const ACTIONS: [Action; 5] = [
    Action::Left,
    Action::Rotate,
    Action::Right,
    Action::Drop,
    Action::Restart,
];

struct Session {
    game: Game,
    gravity: f32,
    repeat: f32,
    keys: [bool; 2],
    pressed: [bool; 5],
    actions: Vec<Action>,
    background: Handle<UiNode>,
    root: Handle<UiNode>,
    cells: Vec<Handle<UiNode>>,
    score: Handle<UiNode>,
    buttons: [Handle<UiNode>; 5],
    scene: Handle<Scene>,
    sound: Handle<Node>,
}

impl Default for Session {
    fn default() -> Self {
        Self {
            game: Game::new(seed()),
            gravity: 0.0,
            repeat: 0.0,
            keys: [false; 2],
            pressed: [false; 5],
            actions: Vec::new(),
            background: Handle::NONE,
            root: Handle::NONE,
            cells: Vec::new(),
            score: Handle::NONE,
            buttons: [Handle::NONE; 5],
            scene: Handle::NONE,
            sound: Handle::NONE,
        }
    }
}

impl Session {
    fn advance(&mut self, dt: f32, mut held: [bool; 2]) -> u32 {
        held[0] &= !self.actions.contains(&Action::Left);
        held[1] &= !self.actions.contains(&Action::Right);
        if held.iter().any(|&pressed| pressed) {
            self.repeat += dt;
            if self.repeat >= 0.12 {
                self.repeat %= 0.12;
                for (pressed, action) in held.into_iter().zip([Action::Left, Action::Right]) {
                    if pressed {
                        self.actions.push(action);
                    }
                }
            }
        } else {
            self.repeat = 0.0;
        }
        let mut cleared = 0;
        for action in self.actions.drain(..) {
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
                Action::Restart => {
                    self.game = Game::new(seed());
                    self.gravity = 0.0;
                    self.repeat = 0.0;
                }
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

#[derive(Default, Visit, Reflect)]
#[reflect(non_cloneable)]
struct Tetris {
    #[visit(skip)]
    #[reflect(hidden)]
    session: Session,
}

impl std::fmt::Debug for Tetris {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Tetris")
            .field("score", &self.session.game.score)
            .finish()
    }
}

fn seed() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64
}

fn brush(color: Color) -> Brush {
    Brush::Solid(color)
}

fn widget(x: f32, y: f32, width: f32, height: f32) -> WidgetBuilder {
    WidgetBuilder::new()
        .with_desired_position(Vector2::new(x, y))
        .with_width(width)
        .with_height(height)
}

fn rectangle(ctx: &mut BuildContext, bounds: [f32; 4], color: Color) -> Handle<UiNode> {
    BorderBuilder::new(
        widget(bounds[0], bounds[1], bounds[2], bounds[3]).with_background(brush(color).into()),
    )
    .with_stroke_thickness(Thickness::zero().into())
    .build(ctx)
    .to_base()
}

fn text(
    ctx: &mut BuildContext,
    font: &FontResource,
    label: &str,
    y: f32,
    height: f32,
    size: f32,
) -> Handle<UiNode> {
    TextBuilder::new(widget(0.0, y, 340.0, height).with_foreground(brush(Color::WHITE).into()))
        .with_font(font.clone())
        .with_font_size(size.into())
        .with_horizontal_text_alignment(HorizontalAlignment::Center)
        .with_vertical_text_alignment(VerticalAlignment::Center)
        .with_text(label)
        .build(ctx)
        .to_base()
}

impl Plugin for Tetris {
    fn init(&mut self, _: Option<&str>, context: PluginContext) -> GameResult {
        context
            .user_interfaces
            .add(UserInterface::new(Vector2::new(400.0, 800.0)));
        let font = Resource::new_ok(
            uuid!("2c358af8-91cc-4e43-8cfa-cb6c2f2c6495"),
            ResourceKind::Embedded,
            Font::from_memory(
                include_bytes!("../assets/FiraMono-subset.ttf").as_slice(),
                512,
                FontStyles::default(),
                Vec::new(),
            )
            .expect("bundled font is valid"),
        );
        let ctx = &mut context.user_interfaces.first_mut().build_ctx();
        self.session.background =
            rectangle(ctx, [0.0, 0.0, 400.0, 800.0], Color::opaque(13, 15, 23));
        let mut children = vec![
            text(ctx, &font, "Tetris", 0.0, 33.6, 28.0),
            rectangle(ctx, [30.0, 77.6, 280.0, 560.0], Color::opaque(56, 64, 82)),
        ];
        self.session.score = text(ctx, &font, "Score: 0", 43.6, 24.0, 20.0);
        children.push(self.session.score);
        for y in 0..HEIGHT {
            for x in 0..WIDTH {
                let cell = rectangle(
                    ctx,
                    [31.0 + x as f32 * 28.0, 78.6 + y as f32 * 28.0, 26.0, 26.0],
                    COLORS[0],
                );
                self.session.cells.push(cell);
                children.push(cell);
            }
        }
        for (i, label) in ["<", "Rotate", ">", "Drop", "Restart"]
            .into_iter()
            .enumerate()
        {
            let (x, y, width, size) = if i == 4 {
                (115.0, 701.6, 110.0, 18.0)
            } else {
                (45.0 + i as f32 * 64.0, 647.6, 58.0, 15.0)
            };
            let back = DecoratorBuilder::new(
                BorderBuilder::new(WidgetBuilder::new())
                    .with_stroke_thickness(Thickness::zero().into()),
            )
            .with_normal_brush(brush(Color::opaque(51, 61, 82)).into())
            .with_hover_brush(brush(Color::opaque(71, 87, 112)).into())
            .with_pressed_brush(brush(Color::opaque(102, 122, 153)).into())
            .build(ctx);
            let button = ButtonBuilder::new(widget(x, y, width, 44.0))
                .with_text_and_font_size(label, font.clone(), size.into())
                .with_back(back)
                .build(ctx);
            let content = *ctx[button].content;
            ctx[content].set_foreground(brush(Color::WHITE));
            let button = button.to_base();
            self.session.buttons[i] = button;
            children.push(button);
        }
        children.push(text(
            ctx,
            &font,
            "Left/Right: move | Up: rotate | Down: drop",
            755.6,
            14.4,
            12.0,
        ));
        self.session.root =
            CanvasBuilder::new(widget(0.0, 0.0, 340.0, 780.0).with_children(children))
                .build(ctx)
                .to_base();

        let buffer = SoundBuffer::raw_generic(DataSource::Raw {
            sample_rate: 44_100,
            channel_count: 1,
            samples: (0..5_292)
                .map(|i| (i as f32 * 880.0 * std::f32::consts::TAU / 44_100.0).sin())
                .collect(),
        })
        .expect("generated mono samples are valid");
        let mut scene = Scene::new();
        self.session.sound = SoundBuilder::new(BaseBuilder::new())
            .with_buffer(Some(Resource::new_ok(
                uuid!("0e2c84b5-573d-4d8f-91af-220c8f4c2638"),
                ResourceKind::Embedded,
                buffer,
            )))
            .with_spatial_blend_factor(0.0)
            .with_gain(1.0)
            .build(&mut scene.graph)
            .to_base();
        self.session.scene = context.scenes.add(scene);
        Ok(())
    }

    #[cfg(target_arch = "wasm32")]
    fn on_graphics_context_initialized(&mut self, context: PluginContext) -> GameResult {
        if let fyrox::engine::GraphicsContext::Initialized(graphics) = context.graphics_context {
            // Fyrox attaches the canvas after winit's initial focus attempt.
            graphics.window.focus_window();
        }
        Ok(())
    }

    fn on_os_event(&mut self, event: &Event<()>, context: PluginContext) -> GameResult {
        if let Event::WindowEvent { event, .. } = event {
            match event {
                WindowEvent::KeyboardInput { event, .. } => {
                    let pressed = event.state == ElementState::Pressed;
                    let action = match event.physical_key {
                        PhysicalKey::Code(KeyCode::ArrowLeft) => {
                            self.session.keys[0] = pressed;
                            Some(Action::Left)
                        }
                        PhysicalKey::Code(KeyCode::ArrowRight) => {
                            self.session.keys[1] = pressed;
                            Some(Action::Right)
                        }
                        PhysicalKey::Code(KeyCode::ArrowUp) => Some(Action::Rotate),
                        PhysicalKey::Code(KeyCode::ArrowDown) => Some(Action::Drop),
                        PhysicalKey::Code(KeyCode::KeyR) => Some(Action::Restart),
                        _ => None,
                    };
                    if pressed
                        && !event.repeat
                        && let Some(action) = action
                    {
                        self.session.actions.push(action);
                    }
                }
                WindowEvent::Focused(false) => {
                    self.session.keys.fill(false);
                    self.session.pressed.fill(false);
                    context.user_interfaces.first_mut().release_mouse_capture();
                }
                _ => {}
            }
        }
        Ok(())
    }

    fn on_ui_message(
        &mut self,
        context: &mut PluginContext,
        message: &UiMessage,
        _: Handle<UserInterface>,
    ) -> GameResult {
        if message.direction() == MessageDirection::FromWidget
            && let Some(event) = message.data::<WidgetMessage>()
        {
            let ui = context.user_interfaces.first_mut();
            for (i, &button) in self.session.buttons.iter().enumerate() {
                if message.destination() == button
                    || ui[button].has_descendant(message.destination(), ui)
                {
                    match event {
                        WidgetMessage::MouseDown {
                            button: MouseButton::Left,
                            ..
                        }
                        | WidgetMessage::TouchStarted { .. } => {
                            if !self.session.pressed[i] {
                                self.session.actions.push(ACTIONS[i]);
                            }
                            self.session.pressed[i] = true;
                        }
                        WidgetMessage::MouseUp {
                            button: MouseButton::Left,
                            ..
                        }
                        | WidgetMessage::TouchEnded { .. }
                        | WidgetMessage::TouchCancelled { .. } => {
                            self.session.pressed[i] = false;
                        }
                        _ => {}
                    }
                }
            }
            if matches!(event, WidgetMessage::TouchCancelled { .. }) {
                ui.release_mouse_capture();
            }
        }
        Ok(())
    }

    fn update(&mut self, context: &mut PluginContext) -> GameResult {
        let session = &mut self.session;
        let ui = context.user_interfaces.first();
        let pointer = ui.cursor_position();
        let held = [
            session.keys[0]
                || (session.pressed[0] && ui[session.buttons[0]].screen_bounds().contains(pointer)),
            session.keys[1]
                || (session.pressed[2] && ui[session.buttons[2]].screen_bounds().contains(pointer)),
        ];
        if session.advance(context.dt, held) > 0 {
            let sound = context.scenes[session.scene].graph[session.sound].as_sound_mut();
            sound.set_playback_time(0.0);
            sound.play();
        }

        let ui = context.user_interfaces.first_mut();
        let size = ui.screen_size();
        ui.send(session.background, WidgetMessage::Width(size.x));
        ui.send(session.background, WidgetMessage::Height(size.y));
        let scale = (size.x / 360.0).min(size.y / 800.0).max(0.1);
        ui.send(
            session.root,
            WidgetMessage::LayoutTransform(Matrix3::new_scaling(scale)),
        );
        ui.send(
            session.root,
            WidgetMessage::DesiredPosition((size - Vector2::new(340.0, 780.0) * scale) / 2.0),
        );
        let blocks = session.game.blocks();
        for (i, &cell) in session.cells.iter().enumerate() {
            let (x, y) = (i % WIDTH, i / WIDTH);
            let value = if !session.game.over && blocks.contains(&(x as i32, y as i32)) {
                session.game.color()
            } else {
                session.game.board[y][x]
            };
            let color = brush(COLORS[value as usize]);
            if ui[cell].background() != color {
                ui.send(cell, WidgetMessage::Background(color.into()));
            }
        }
        let label = if session.game.over {
            format!("Game over | Score: {}", session.game.score)
        } else {
            format!("Score: {}", session.game.score)
        };
        if ui[session.score]
            .cast::<fyrox::gui::text::Text>()
            .unwrap()
            .text()
            != label
        {
            ui.send(session.score, TextMessage::Text(label));
        }
        Ok(())
    }
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen::prelude::wasm_bindgen)]
pub fn start() {
    let mut event_loop = EventLoop::builder();
    #[cfg(target_os = "linux")]
    if std::env::var_os("WSL_DISTRO_NAME").is_some() && std::env::var_os("DISPLAY").is_some() {
        use fyrox::platform::x11::EventLoopBuilderExtX11;
        // Fyrox's Wayland window can restart the WSLg compositor.
        event_loop.with_x11();
    }
    let attributes = WindowAttributes::default()
        .with_title("Tetris - Fyrox")
        .with_inner_size(LogicalSize::new(400.0, 800.0));
    let mut executor = Executor::from_params(
        Some(event_loop.build().expect("event loop is available")),
        GraphicsContextParams {
            window_attributes: attributes,
            ..Default::default()
        },
    );
    executor.set_resource_hot_reloading_enabled(false);
    // All assets are embedded; the browser needs only an empty resource registry.
    #[cfg(target_arch = "wasm32")]
    executor
        .resource_manager
        .state()
        .resource_registry
        .lock()
        .set_path("data:text/plain,%7B%7D");
    executor.add_plugin(Tetris::default());
    executor.run();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plugin_initializes_and_updates_an_empty_engine_ui_container() {
        let mut executor = Executor::new(None);
        let engine = &mut *executor;
        let running = std::cell::Cell::new(true);
        let mut lag = 0.0;
        let mut game = Tetris::default();
        let mut context = PluginContext {
            scenes: &mut engine.scenes,
            resource_manager: &engine.resource_manager,
            user_interfaces: &mut engine.user_interfaces,
            graphics_context: &mut engine.graphics_context,
            dt: 0.0,
            lag: &mut lag,
            serialization_context: &engine.serialization_context,
            widget_constructors: &engine.widget_constructors,
            dyn_type_constructors: &engine.dyn_type_constructors,
            performance_statistics: &Default::default(),
            elapsed_time: 0.0,
            script_processor: &engine.script_processor,
            loop_controller: fyrox::engine::ApplicationLoopController::Headless {
                running: &running,
            },
            task_pool: &mut engine.task_pool,
            input_state: &Default::default(),
        };
        assert_eq!(context.user_interfaces.iter().count(), 0);
        game.init(
            None,
            PluginContext {
                scenes: &mut *context.scenes,
                user_interfaces: &mut *context.user_interfaces,
                graphics_context: &mut *context.graphics_context,
                lag: &mut *context.lag,
                task_pool: &mut *context.task_pool,
                ..context
            },
        )
        .unwrap();
        game.update(&mut context).unwrap();
        assert_eq!(context.user_interfaces.iter().count(), 1);
        assert_eq!(game.session.cells.len(), WIDTH * HEIGHT);
    }

    #[test]
    fn movement_repeats_only_after_a_fresh_press_and_restart_resets_timers() {
        let mut session = Session::default();
        let initial_x = session.game.blocks()[0].0;
        session.actions.push(Action::Left);
        session.advance(0.05, [true, false]);
        assert_eq!(session.game.blocks()[0].0, initial_x - 1);
        assert_eq!(session.repeat, 0.0);
        session.advance(0.10, [true, false]);
        assert_eq!(session.game.blocks()[0].0, initial_x - 1);
        session.advance(0.03, [true, false]);
        assert_eq!(session.game.blocks()[0].0, initial_x - 2);
        session.actions.push(Action::Restart);
        session.advance(0.0, [false, false]);
        assert_eq!((session.gravity, session.repeat), (0.0, 0.0));
        assert!(session.game.board.iter().flatten().all(|&cell| cell == 0));
        session.game.over = true;
        session.advance(1.0, [false, false]);
        assert_eq!(session.gravity, 0.0);
    }
}
