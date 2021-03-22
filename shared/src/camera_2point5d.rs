use bevy::{
    app,
    prelude::*,
    render::{
        camera,
        camera::{Camera, CameraProjection, DepthCalculation, VisibleEntities},
        render_graph::base::camera::CAMERA_3D,
    },
};

pub struct Camera2point5dPlugin;

impl Plugin for Camera2point5dPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.register_type::<ZBasedPerspectiveProjection>()
            .add_system_to_stage(
                app::stage::POST_UPDATE,
                camera::camera_system::<ZBasedPerspectiveProjection>.system(),
            );
    }
}

#[derive(Debug, Clone, Reflect)]
#[reflect(Component)]
pub struct ZBasedPerspectiveProjection {
    pub fov: f32,
    pub aspect_ratio: f32,
    pub near: f32,
    pub far: f32,
}

impl CameraProjection for ZBasedPerspectiveProjection {
    fn get_projection_matrix(&self) -> Mat4 {
        Mat4::perspective_rh(self.fov, self.aspect_ratio, self.near, self.far)
    }

    fn update(&mut self, width: f32, height: f32) {
        self.aspect_ratio = width / height;
    }

    fn depth_calculation(&self) -> DepthCalculation {
        DepthCalculation::ZDifference
    }
}

impl Default for ZBasedPerspectiveProjection {
    fn default() -> Self {
        Self {
            fov: std::f32::consts::PI / 4.0,
            near: 1.0,
            far: 1000.0,
            aspect_ratio: 1.0,
        }
    }
}

#[derive(Bundle)]
pub struct Camera2point5dBundle {
    pub camera: Camera,
    pub camera_projection: ZBasedPerspectiveProjection,
    pub visible_entities: VisibleEntities,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
}

impl Default for Camera2point5dBundle {
    fn default() -> Self {
        Self {
            camera: Camera {
                name: Some(CAMERA_3D.to_string()),
                depth_calculation: DepthCalculation::ZDifference,
                ..Default::default()
            },
            camera_projection: Default::default(),
            visible_entities: Default::default(),
            transform: Default::default(),
            global_transform: Default::default(),
        }
    }
}
