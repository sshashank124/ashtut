use std::collections::HashSet;

use winit::{
    event::{ElementState, MouseButton},
    keyboard::KeyCode,
};

#[derive(Default)]
pub struct State {
    keys: HashSet<KeyCode>,
    buttons: HashSet<MouseButton>,
    mouse_delta: glam::Vec2,
}

impl State {
    pub fn handle_key(&mut self, key_code: KeyCode, state: ElementState) {
        match state {
            ElementState::Pressed => self.keys.insert(key_code),
            ElementState::Released => self.keys.remove(&key_code),
        };
    }

    pub fn handle_button(&mut self, button: MouseButton, state: ElementState) {
        match state {
            ElementState::Pressed => self.buttons.insert(button),
            ElementState::Released => self.buttons.remove(&button),
        };
    }

    pub fn handle_mouse_motion(&mut self, delta: (f64, f64)) {
        if self.button_pressed(MouseButton::Right) {
            self.mouse_delta += glam::DVec2::from(delta).as_vec2();
        }
    }

    pub fn key_pressed(&self, key_code: KeyCode) -> bool {
        self.keys.contains(&key_code)
    }

    pub fn button_pressed(&self, button: MouseButton) -> bool {
        self.buttons.contains(&button)
    }

    pub fn mouse_delta(&mut self) -> glam::Vec2 {
        let delta = self.mouse_delta;
        self.mouse_delta = glam::Vec2::ZERO;
        delta
    }
}
