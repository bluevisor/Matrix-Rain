use glam::{Mat4, Vec3};

pub struct Camera {
    pub fov_deg: f32,
    pub aspect: f32,
    pub near: f32,
    pub far: f32,
    pub position: Vec3,
    pub target: Vec3,
}

impl Camera {
    pub fn new(aspect: f32) -> Self {
        Camera {
            fov_deg: 60.0,
            aspect,
            near: 0.1,
            far: 200.0,
            position: Vec3::new(0.0, 0.0, 5.0),
            target: Vec3::new(0.0, 0.0, -50.0),
        }
    }

    pub fn zoom_in(&mut self) {
        self.fov_deg = (self.fov_deg - 2.0).max(20.0);
    }

    pub fn zoom_out(&mut self) {
        self.fov_deg = (self.fov_deg + 2.0).min(120.0);
    }

    pub fn view_proj(&self) -> Mat4 {
        let view = Mat4::look_at_rh(self.position, self.target, Vec3::Y);
        let proj = Mat4::perspective_rh(
            self.fov_deg.to_radians(),
            self.aspect,
            self.near,
            self.far,
        );
        proj * view
    }

    pub fn set_aspect(&mut self, aspect: f32) {
        self.aspect = aspect;
    }
}
