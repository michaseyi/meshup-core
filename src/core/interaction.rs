use bevy::{
    ecs::schedule::common_conditions, math::bounding::RayCast3d, prelude::*, window::PrimaryWindow,
};
use lox::FaceHandle;

use super::{
    editable_mesh::{bvh::BoundingVolumeHierarchy, EditableMesh, SelectMode},
    editor::Focused,
    pan_orbit_camera::{PanOrbitCameraUpdate, PrimaryCamera},
};

use wasm_bindgen::prelude::*;
pub struct InteractionPlugin;

/// After this set runs, intersection caches are updated
#[derive(SystemSet, Clone, Copy, Hash, Debug, PartialEq, Eq)]
pub enum InteractionSet {
    /// Runs intersection tests on the scene, and store the results on hit entity in InteractionCache
    IntersectionTest,

    /// Runs after IntersectionTest, and updates components like ActiveVertices, ActiveEdges, ActiveFaces based on the InteractionCache
    ActivesUpdate,
}

#[derive(Component, Deref, DerefMut, Default)]
pub struct InteractionCache(pub Option<(FaceHandle, Vec3)>);

#[wasm_bindgen]
#[derive(Resource, Clone, Debug, PartialEq, Eq, Copy)]
pub enum InteractionMode {
    Object,
    Edit,
    Sculpt,
}

impl Plugin for InteractionPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(InteractionMode::Object).add_systems(
            Update,
            Self::multi_object_mode_interaction
                .in_set(InteractionSet::IntersectionTest)
                .after(PanOrbitCameraUpdate),
        );
    }
}

impl InteractionPlugin {
    fn multi_object_mode_interaction(
        interaction_mode: Res<InteractionMode>,
        mut commands: Commands,
        focused: Query<
            (
                &BoundingVolumeHierarchy,
                &Transform,
                &Name,
                &EditableMesh,
                Entity,
            ),
            With<Focused>,
        >,
        query: Query<
            (
                &BoundingVolumeHierarchy,
                &Transform,
                &Name,
                &EditableMesh,
                Entity,
            ),
            Without<Focused>,
        >,
        mouse: Res<ButtonInput<MouseButton>>,
        window: Query<&Window, With<PrimaryWindow>>,
        camera: Query<(&Camera, &GlobalTransform), With<PrimaryCamera>>,
    ) {
        if !mouse.just_pressed(MouseButton::Left) {
            return;
        }

        let window = window.single();

        let Some(cursor_position) = window.cursor_position() else {
            return;
        };

        let (camera, global_transform) = camera.single();

        let Some(ray) = camera.viewport_to_world(global_transform, cursor_position) else {
            return;
        };

        let ray_cast = RayCast3d::from_ray(ray, 1000.0);

        let focused_entity = focused.get_single();

        match *interaction_mode {
            InteractionMode::Object => {
                // In object mode, exact face and point is not needed, so we use the fast intersection test which is a rough approximation
                let (mut closest_t, mut closest_entity) = (f32::MAX, None);

                for (bvh, transform, _, _, entity) in query.iter().chain(focused.iter()) {
                    if let Some(t) = bvh.intersects_ray_at_fast(&ray_cast, transform) {
                        if t < closest_t {
                            closest_t = t;
                            closest_entity = Some(entity);
                        }
                    }
                }
                if let Some(entity) = closest_entity {
                    commands.entity(entity).insert(Focused);
                }
                // if let (Ok((_, _, _, _, entity)), None) = (&focused_entity, &closest_entity) {
                //     commands.entity(entity.clone()).remove::<Focused>();
                // }
                if let (Ok((_, _, _, _, entity)), Some(closest_entity)) =
                    (&focused_entity, &closest_entity)
                {
                    if entity != closest_entity {
                        commands.entity(entity.clone()).remove::<Focused>();
                    }
                }
            }
            _ => {
                let (bvh, transform, _, mesh, entity) = focused.get_single().expect("In non-object mode, there should be exactly one focused entity. This is a bug.");

                // In non-object mode, we need the exact face and point that was intersected
                if let Some((face_handle, t)) = bvh.intersects_ray_at(&ray_cast, transform, mesh) {
                    commands.entity(entity).insert(InteractionCache(Some((
                        face_handle,
                        ray_cast.ray.get_point(t),
                    ))));
                } else {
                    commands.entity(entity).insert(InteractionCache(None));
                }
            }
        };
    }
}
