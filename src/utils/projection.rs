use bevy::math::Vec3;

pub fn project_to_plane(
    camera_position: Vec3,
    camera_forward: Vec3,
    object_position: Vec3,
    plane_distance: f32,
) -> Vec3 {
    let camera_to_object = object_position - camera_position;
    let scale_factor = plane_distance / camera_to_object.dot(camera_forward);
    camera_position + camera_to_object * scale_factor
}

/// Returns the angles between the direction vector and the x, y, and z axes.
pub fn compute_orientation_angles(direction: Vec3) -> (f32, f32, f32) {
    let xy = Vec3::new(direction.x, direction.y, 0.0);
    let xz = Vec3::new(direction.x, 0.0, direction.z);
    let yz = Vec3::new(0.0, direction.y, direction.z);

    (
        yz.angle_between(Vec3::Z),
        xz.angle_between(Vec3::Z),
        xy.angle_between(Vec3::Y),
    )
}
