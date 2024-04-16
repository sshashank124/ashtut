use shared::inputs;

mod conf {
    pub const Z_NEAR: f32 = 1e-1;
    pub const Z_FAR: f32 = 1e+4;
    pub const MOVE_SPEED: f32 = 5e-7;
    pub const PAN_SPEED: f32 = 2e-7;
}

pub struct CameraController {
    position: glam::Vec3,
    direction: glam::Vec3,
    aspect_ratio: f32,
    fov: f32,
    scale: f32,
}

pub enum AxisMovement {
    None,
    Forward,
    Backward,
}

impl CameraController {
    pub fn new(
        position: glam::Vec3,
        target: glam::Vec3,
        resolution: (u32, u32),
        fov_deg: f32,
    ) -> Self {
        let direction = target - position;
        let scale = direction.length();
        Self {
            position,
            direction: direction.normalize(),
            aspect_ratio: resolution.0 as f32 / resolution.1 as f32,
            fov: fov_deg.to_radians(),
            scale,
        }
    }

    pub fn move_in_direction(&mut self, axes: &[AxisMovement; 3], slow: bool, delta_us: u128) {
        let final_direction = axes[0].factor() * self.direction
            + axes[1].factor() * self.right_axis()
            + axes[2].factor() * glam::Vec3::Y;

        let slow_factor = if slow { 0.1 } else { 1. };

        self.position += (conf::MOVE_SPEED * self.scale * slow_factor * delta_us as f32)
            * final_direction.normalize_or_zero();
    }

    pub fn pan(&mut self, mouse_delta: glam::Vec2, delta_us: u128) {
        let pan = conf::PAN_SPEED * -mouse_delta * delta_us as f32;
        self.direction = glam::Mat3::from_rotation_y(pan.x)
            * glam::Mat3::from_axis_angle(self.right_axis(), pan.y)
            * self.direction;
    }

    fn right_axis(&self) -> glam::Vec3 {
        glam::vec3(-self.direction.z, 0., self.direction.x)
    }

    pub fn camera(&self) -> inputs::Camera {
        inputs::Camera {
            view: inputs::Transform::new(glam::Mat4::look_to_rh(
                self.position,
                self.direction,
                glam::Vec3::Y,
            )),
            proj: inputs::Transform::proj(glam::Mat4::perspective_rh(
                self.fov,
                self.aspect_ratio,
                conf::Z_NEAR,
                conf::Z_FAR,
            )),
        }
    }
}

impl AxisMovement {
    pub const fn new(forward: bool, backward: bool) -> Self {
        match (forward, backward) {
            (true, false) => Self::Forward,
            (false, true) => Self::Backward,
            _ => Self::None,
        }
    }

    pub const fn has_some(&self) -> bool {
        !matches!(self, Self::None)
    }

    const fn factor(&self) -> f32 {
        match self {
            Self::None => 0.,
            Self::Backward => -1.,
            Self::Forward => 1.,
        }
    }
}
