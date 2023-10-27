use shared::glam;

#[derive(Clone, Copy, Debug)]
pub struct BoundingBox {
    pub min: glam::Vec3,
    pub max: glam::Vec3,
}

impl BoundingBox {
    pub fn new<T: Into<glam::Vec3>>(min: T, max: T) -> Self {
        Self {
            min: min.into(),
            max: max.into(),
        }
    }

    pub fn transform(self, transform: glam::Mat4) -> Self {
        let a = (transform * self.min.extend(1.0)).truncate();
        let b = (transform * self.max.extend(1.0)).truncate();
        Self::new(a.min(b), a.max(b))
    }

    pub fn union(self, other: Self) -> Self {
        Self::new(self.min.min(other.min), self.max.max(other.max))
    }

    pub fn center(&self) -> glam::Vec3 {
        (self.min + self.max) / 2.
    }

    pub fn size(&self) -> glam::Vec3 {
        self.max - self.min
    }
}

impl Default for BoundingBox {
    fn default() -> Self {
        Self::new(glam::Vec3::INFINITY, glam::Vec3::NEG_INFINITY)
    }
}
