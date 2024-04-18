use std::time::Instant;

use winit::{
    dpi::PhysicalSize,
    event::{DeviceEvent, ElementState, Event, KeyEvent, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    keyboard::{Key, KeyCode, NamedKey, PhysicalKey},
    window::{Window, WindowBuilder},
};

use renderer::{Error as RendererError, Renderer};

use crate::{
    data::camera_controller::{AxisMovement, CameraController},
    input,
};

pub mod conf {
    pub const FRAME_RESOLUTION: (u32, u32) = (1600, 1200);
    pub const FOV_DEGREES: f32 = 45.;
}

pub struct App {
    renderer: Renderer,

    // state
    last_frame: Instant,
    inputs: input::State,
    camera_controller: CameraController,
    needs_resizing: bool,
}

impl App {
    pub fn new(window: &Window, scene_file: &str) -> Self {
        let scene = scene::io::load(scene_file);

        let camera_controller = CameraController::new(
            scene.info.bounding_box.size() * 1.2 + scene.info.bounding_box.center(),
            scene.info.bounding_box.center(),
            conf::FRAME_RESOLUTION,
            conf::FOV_DEGREES,
        );

        let inputs = input::State::default();

        let renderer = Renderer::create(
            &window.title(),
            window,
            scene,
            conf::FRAME_RESOLUTION,
            camera_controller.camera(),
        );

        Self {
            renderer,

            last_frame: Instant::now(),
            inputs,
            camera_controller,
            needs_resizing: false,
        }
    }

    fn render(&mut self) {
        self.update();

        if self.needs_resizing {
            if self.renderer.recreate() {
                self.needs_resizing = false;
            } else {
                return;
            }
        }

        if matches!(self.renderer.render(), Err(RendererError::NeedsRecreating)) {
            self.needs_resizing = true;
        }
    }

    fn update(&mut self) {
        let now = Instant::now();
        let delta_us = (now - self.last_frame).as_micros();

        let moves = [
            AxisMovement::new(
                self.inputs.key_pressed(KeyCode::KeyW),
                self.inputs.key_pressed(KeyCode::KeyS),
            ),
            AxisMovement::new(
                self.inputs.key_pressed(KeyCode::KeyD),
                self.inputs.key_pressed(KeyCode::KeyA),
            ),
            AxisMovement::new(
                self.inputs.key_pressed(KeyCode::Space),
                self.inputs.key_pressed(KeyCode::ShiftLeft),
            ),
        ];

        let has_movement = moves.iter().any(AxisMovement::has_some);
        if has_movement {
            let slow_move = self.inputs.key_pressed(KeyCode::ControlLeft);
            self.camera_controller
                .move_in_direction(&moves, slow_move, delta_us);
        }

        let mouse_delta = self.inputs.mouse_delta();
        let has_rotation = mouse_delta.length_squared() > 0.;
        if has_rotation {
            self.camera_controller.pan(mouse_delta, delta_us);
        }

        if has_movement || has_rotation {
            self.renderer.update_camera(self.camera_controller.camera());
        }

        self.last_frame = now;
    }

    pub fn window_builder() -> WindowBuilder {
        WindowBuilder::new().with_inner_size(PhysicalSize::<u32>::from(conf::FRAME_RESOLUTION))
    }

    pub fn run(mut self, event_loop: EventLoop<()>) {
        event_loop.set_control_flow(ControlFlow::Poll);
        event_loop
            .run(move |event, elwt| match event {
                Event::AboutToWait => self.render(),
                Event::WindowEvent { ref event, .. } => match event {
                    WindowEvent::Resized(_) | WindowEvent::ScaleFactorChanged { .. } => {
                        self.needs_resizing = true;
                    }
                    WindowEvent::CloseRequested
                    | WindowEvent::KeyboardInput {
                        event:
                            KeyEvent {
                                logical_key: Key::Named(NamedKey::Escape),
                                state: ElementState::Pressed,
                                repeat: false,
                                ..
                            },
                        ..
                    } => elwt.exit(),
                    WindowEvent::KeyboardInput {
                        event:
                            KeyEvent {
                                physical_key: PhysicalKey::Code(KeyCode::KeyE),
                                state: ElementState::Pressed,
                                repeat: false,
                                ..
                            },
                        ..
                    } => self.renderer.toggle_renderer(),
                    WindowEvent::KeyboardInput {
                        event:
                            KeyEvent {
                                physical_key: PhysicalKey::Code(key_code),
                                state,
                                ..
                            },
                        ..
                    } => self.inputs.handle_key(*key_code, *state),
                    WindowEvent::MouseInput { button, state, .. } => {
                        self.inputs.handle_button(*button, *state);
                    }
                    _ => (),
                },
                Event::DeviceEvent {
                    event: DeviceEvent::MouseMotion { delta },
                    ..
                } => self.inputs.handle_mouse_motion(delta),
                _ => (),
            })
            .expect("Error running the event loop");
    }
}
