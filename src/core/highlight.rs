use std::{fmt::Debug, marker::PhantomData};

use bevy::{
    core_pipeline::core_3d::Transparent3d,
    pbr::{
        DrawMesh, ExtendedMaterial, MaterialPipeline, MaterialPipelineKey, MeshPipeline,
        MeshPipelineKey, SetMaterialBindGroup, SetMeshBindGroup, SetMeshViewBindGroup,
        StandardMaterialUniform,
    },
    prelude::*,
    render::{
        extract_component::{ComponentUniforms, ExtractComponent, ExtractComponentPlugin},
        render_phase::{DrawFunctionId, PhaseItem, RenderCommand, RenderPhase, SetItemPipeline},
        render_resource::{
            CachedRenderPipelineId, PipelineCache, SpecializedMeshPipeline,
            SpecializedMeshPipelines,
        },
        view::ExtractedView,
        Render, RenderApp, RenderSet,
    },
};

use super::pan_orbit_camera::PrimaryCamera;

#[derive(Default)]
pub struct HighlightPlugin<T: Material>(pub PhantomData<T>);

#[derive(Component)]
pub struct Highlight;

#[derive(Component, Deref, DerefMut, Clone, Debug)]
pub struct HighlightTarget<T: Material + Debug>(pub Handle<T>);

impl<T: Material + Debug> ExtractComponent for HighlightTarget<T> {
    type Out = Self;
    type QueryData = &'static Self;
    type QueryFilter = ();

    fn extract_component(item: &Self) -> Option<Self::Out> {
        Some(item.clone())
    }
}

impl<T: Material + Debug> Plugin for HighlightPlugin<T> {
    fn build(&self, app: &mut App) {
        app.add_plugins(ExtractComponentPlugin::<HighlightTarget<T>>::extract_visible())
            .add_systems(
                Update,
                (
                    Self::extract_highlighted_entities,
                    Self::handle_removed_highlight,
                ),
            )
            .sub_app_mut(RenderApp)
            .add_systems(
                Render,
                (
                    Self::prepare_pipeline.in_set(RenderSet::Prepare),
                    Self::queue_highlighted_meshes.in_set(RenderSet::QueueMeshes),
                ),
            );
    }
}
impl<T: Material + Debug> HighlightPlugin<T> {
    pub fn extract_highlighted_entities(
        mut commands: Commands,
        query: Query<(Entity, &Handle<T>), With<Highlight>>,
    ) {
        for (entity, handle) in query.iter() {
            commands
                .entity(entity)
                .insert(HighlightTarget(handle.clone()))
                .remove::<Handle<T>>();
        }
    }

    pub fn handle_removed_highlight(
        mut commands: Commands,
        query: Query<(Entity, &HighlightTarget<T>), Without<Highlight>>,
    ) {
        for (entity, highlight_target) in query.iter() {
            commands
                .entity(entity)
                .insert((**highlight_target).clone())
                .remove::<HighlightTarget<T>>();
        }
    }

    pub fn queue_highlighted_meshes(
        mut pipelines: ResMut<SpecializedMeshPipelines<PreHighlightPipeline<T>>>,
        pipeline_cache: Res<PipelineCache>,
        highlight_pipeline: Res<PreHighlightPipeline<T>>,
        highlighted: Query<(Entity, &HighlightTarget<T>)>,
        views: Query<(&ExtractedView, &mut RenderPhase<Transparent3d>), With<PrimaryCamera>>,
    ) {
        for (view, mut transparent_phase) in views.iter() {
            for (entity, highlight_target) in highlighted.iter() {
                // queue mesh

                // transparent_phase.add(Transparent3d {
                //     batch_range: 0..1,
                //     distance: 0.0,
                //     draw_function: DrawFunctionId::,
                //     dynamic_offset: None,
                //     entity,
                //     pipeline: CachedRenderPipelineId::INVALID,
                // });
            }
        }
    }

    pub fn prepare_pipeline(query: Query<(Entity, &HighlightTarget<T>)>) {
        for (_, highlight_target) in query.iter() {
            // prepare pipeline
        }
    }
}

#[derive(Resource)]
struct PreHighlightPipeline<T: Material> {
    material_pipeline: MaterialPipeline<T>,
}

impl<T: Material> FromWorld for PreHighlightPipeline<T> {
    fn from_world(world: &mut World) -> Self {
        unimplemented!()
    }
}

impl<T: Material> SpecializedMeshPipeline for PreHighlightPipeline<T> {
    type Key = MeshPipelineKey;

    fn specialize(
        &self,
        _key: Self::Key,
        _layout: &bevy::render::mesh::MeshVertexBufferLayout,
    ) -> Result<
        bevy::render::render_resource::RenderPipelineDescriptor,
        bevy::render::render_resource::SpecializedMeshPipelineError,
    > {
        unimplemented!()
    }
}

#[derive(Resource)]
struct PostHighlightPipeline {
    material_pipeline: MeshPipeline,
}

impl FromWorld for PostHighlightPipeline {
    fn from_world(world: &mut World) -> Self {
        unimplemented!()
    }
}

impl SpecializedMeshPipeline for PostHighlightPipeline {
    type Key = MeshPipelineKey;

    fn specialize(
        &self,
        _key: Self::Key,
        _layout: &bevy::render::mesh::MeshVertexBufferLayout,
    ) -> Result<
        bevy::render::render_resource::RenderPipelineDescriptor,
        bevy::render::render_resource::SpecializedMeshPipelineError,
    > {
        unimplemented!()
    }
}

type DrawHighlightedMesh<T> = (
    SetPreHighlightPipeline,
    SetMeshViewBindGroup<0>,
    SetMeshBindGroup<1>,
    SetMaterialBindGroup<T, 2>,
    DrawMesh,
    SetPostHighlightPipeline,
    DrawHighlight,
);

struct SetPreHighlightPipeline;

impl<T: PhaseItem> RenderCommand<T> for SetPreHighlightPipeline {
    type ItemQuery = ();
    type Param = ();
    type ViewQuery = ();

    fn render<'w>(
        item: &T,
        view: bevy::ecs::query::ROQueryItem<'w, Self::ViewQuery>,
        entity: Option<bevy::ecs::query::ROQueryItem<'w, Self::ItemQuery>>,
        param: bevy::ecs::system::SystemParamItem<'w, '_, Self::Param>,
        pass: &mut bevy::render::render_phase::TrackedRenderPass<'w>,
    ) -> bevy::render::render_phase::RenderCommandResult {
        unimplemented!()
    }
}

struct SetPostHighlightPipeline;

impl<T: PhaseItem> RenderCommand<T> for SetPostHighlightPipeline {
    type ItemQuery = ();
    type Param = ();
    type ViewQuery = ();

    fn render<'w>(
        item: &T,
        view: bevy::ecs::query::ROQueryItem<'w, Self::ViewQuery>,
        entity: Option<bevy::ecs::query::ROQueryItem<'w, Self::ItemQuery>>,
        param: bevy::ecs::system::SystemParamItem<'w, '_, Self::Param>,
        pass: &mut bevy::render::render_phase::TrackedRenderPass<'w>,
    ) -> bevy::render::render_phase::RenderCommandResult {
        unimplemented!()
    }
}

struct DrawHighlight;

impl<T: PhaseItem> RenderCommand<T> for DrawHighlight {
    type ItemQuery = ();
    type Param = ();
    type ViewQuery = ();

    fn render<'w>(
        item: &T,
        view: bevy::ecs::query::ROQueryItem<'w, Self::ViewQuery>,
        entity: Option<bevy::ecs::query::ROQueryItem<'w, Self::ItemQuery>>,
        param: bevy::ecs::system::SystemParamItem<'w, '_, Self::Param>,
        pass: &mut bevy::render::render_phase::TrackedRenderPass<'w>,
    ) -> bevy::render::render_phase::RenderCommandResult {
        unimplemented!()
    }
}
