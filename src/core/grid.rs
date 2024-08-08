use std::num::{NonZero, NonZeroU32};

use bevy::{
    core_pipeline::core_3d::Transparent3d,
    ecs::{
        query::{QueryItem, QuerySingleError},
        system::lifetimeless::{Read, SRes},
    },
    pbr::{
        ExtendedMaterial, MeshPipeline, MeshPipelineKey, MeshUniform, RenderMeshInstances,
        SetMeshBindGroup, SetMeshViewBindGroup,
    },
    prelude::*,
    render::{
        batching::GetBatchData,
        camera::CameraProjection,
        extract_component::{ExtractComponent, ExtractComponentPlugin},
        mesh::{GpuBufferInfo, Indices, PrimitiveTopology},
        primitives::Frustum,
        render_asset::{RenderAssetUsages, RenderAssets},
        render_phase::{
            AddRenderCommand, DrawFunctions, PhaseItem, RenderCommand, RenderCommandResult,
            RenderPhase, SetItemPipeline,
        },
        render_resource::{
            AsBindGroup, BindGroup, BindGroupEntry, BindGroupLayout, BindGroupLayoutEntry,
            BindingType, Buffer, BufferBindingType, BufferInitDescriptor, BufferUsages,
            GpuArrayBuffer, PipelineCache, ShaderRef, ShaderStage, ShaderStages, ShaderType,
            SpecializedMeshPipeline, SpecializedMeshPipelines, UniformBuffer, VertexAttribute,
            VertexBufferLayout, VertexFormat, VertexStepMode,
        },
        renderer::{RenderDevice, RenderQueue},
        view::{ExtractedView, NoFrustumCulling},
        Render, RenderApp, RenderSet,
    },
    utils::nonmax::NonMaxU32,
};
use bytemuck::{Pod, Zeroable};

use super::pan_orbit_camera::{PanOrbitState, PrimaryCamera};

#[derive(Clone, Copy, Default, Pod, Zeroable)]
#[repr(C)]
struct GridInstanceData {
    pub position: Vec3,
    pub scale: f32,
}

#[derive(Component, Deref, DerefMut, Default)]
struct GridInstances(Vec<GridInstanceData>);

#[derive(Component)]
struct InstanceBuffer {
    buffer: Buffer,
    length: usize,
}

impl ExtractComponent for GridInstances {
    type QueryData = Ref<'static, GridInstances>;
    type QueryFilter = ();
    type Out = Self;

    fn extract_component(item: QueryItem<'_, Self::QueryData>) -> Option<Self> {
        Some(GridInstances(item.0.clone()))
    }
}

#[derive(Component, Default)]
pub struct Grid3d;

pub struct GridPlugin;

impl ExtractComponent for PrimaryCamera {
    type QueryData = ();
    type QueryFilter = With<PrimaryCamera>;
    type Out = Self;

    fn extract_component(_item: QueryItem<'_, Self::QueryData>) -> Option<Self> {
        Some(PrimaryCamera)
    }
}

impl Plugin for GridPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(ExtractComponentPlugin::<GridInstances>::default())
            .add_plugins(ExtractComponentPlugin::<PrimaryCamera>::default())
            .add_systems(Update, Self::update)
            .add_systems(Startup, Self::setup_main_world)
            .sub_app_mut(RenderApp)
            .add_render_command::<Transparent3d, DrawGrid>()
            .init_resource::<SpecializedMeshPipelines<GridPipeline>>()
            .add_systems(
                Render,
                (
                    Self::queue_custom.in_set(RenderSet::QueueMeshes),
                    Self::prepare_instance_buffers.in_set(RenderSet::PrepareResources),
                    Self::extract_model_index
                        .after(RenderSet::PrepareResources)
                        .before(RenderSet::PrepareResourcesFlush),
                ),
            );
    }

    fn finish(&self, app: &mut App) {
        app.sub_app_mut(RenderApp).init_resource::<GridPipeline>();
    }
}

#[derive(AsBindGroup, ShaderType, Default, Zeroable, Pod, Copy, Clone)]
#[repr(C)]
struct GridData {
    _padding: UVec3,
    #[uniform(0)]
    model_index: u32,
}

impl GridPlugin {
    fn prepare_instance_buffers(
        mut commands: Commands,
        query: Query<(Entity, Ref<GridInstances>)>,
        render_device: Res<RenderDevice>,
        pipeline: Res<GridPipeline>,
    ) {
        for (entity, grid_instance_data) in &query {
            let buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
                label: Some("Grid Instance Data Buffer"),
                contents: bytemuck::cast_slice(grid_instance_data.as_slice()),
                usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
            });

            commands.entity(entity).insert((
                InstanceBuffer {
                    buffer,
                    length: grid_instance_data.len(),
                },
                GridUniformBindGroup {
                    bind_group: pipeline.bind_group.clone(),
                },
            ));
        }
    }

    fn extract_model_index(
        grid: Query<Entity, (With<GridUniformBindGroup>, Without<PrimaryCamera>)>,
        views: Query<&RenderPhase<Transparent3d>, With<PrimaryCamera>>,
        mut pipeline: ResMut<GridPipeline>,
        render_device: Res<RenderDevice>,
        render_queue: Res<RenderQueue>,
    ) {
        let Ok(entity) = grid.get_single() else {
            return;
        };

        // This is a bit of a hack to extract the batch range storing the entity model index. It is temporary util i can figure out our to
        // update the model index from the draw functions.
        for phase in &views {
            for item in phase.items.iter() {
                if item.entity() == entity {
                    let mut range = item.batch_range().clone();
                    let model_index = range.next().unwrap();
                    pipeline.uniform.set(GridData {
                        model_index,
                        ..Default::default()
                    });

                    pipeline
                        .uniform
                        .write_buffer(&*render_device, &*render_queue);
                }
            }
        }
    }
    fn queue_custom(
        transparent_3d_draw_functions: Res<DrawFunctions<Transparent3d>>,
        grid_pipeline: Res<GridPipeline>,
        msaa: Res<Msaa>,
        mut pipelines: ResMut<SpecializedMeshPipelines<GridPipeline>>,
        pipeline_cache: Res<PipelineCache>,
        meshes: Res<RenderAssets<Mesh>>,
        render_mesh_instances: Res<RenderMeshInstances>,
        material_meshes: Query<Entity, With<GridInstances>>,
        mut views: Query<
            (&ExtractedView, &mut RenderPhase<Transparent3d>, Entity),
            With<PrimaryCamera>,
        >,
    ) {
        let draw_custom = transparent_3d_draw_functions.read().id::<DrawGrid>();
        let msaa_key = MeshPipelineKey::from_msaa_samples(msaa.samples());

        for (view, mut transparent_phase, entity) in &mut views {
            let view_key = msaa_key | MeshPipelineKey::from_hdr(view.hdr);

            let rangefinder = view.rangefinder3d();

            for entity in &material_meshes {
                let Some(mesh_instance) = render_mesh_instances.get(&entity) else {
                    continue;
                };
                let Some(mesh) = meshes.get(mesh_instance.mesh_asset_id) else {
                    continue;
                };

                let key = view_key
                    | MeshPipelineKey::from_primitive_topology(mesh.primitive_topology)
                    | MeshPipelineKey::BLEND_ALPHA;

                let pipeline = pipelines
                    .specialize(&pipeline_cache, &grid_pipeline, key, &mesh.layout)
                    .unwrap();

                transparent_phase.add(Transparent3d {
                    entity,
                    pipeline,
                    draw_function: draw_custom,
                    distance: rangefinder
                        .distance_translation(&mesh_instance.transforms.transform.translation),
                    batch_range: 0..1,
                    dynamic_offset: None,
                });
            }
        }
    }

    fn setup_main_world(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>) {
        let grid_size = 30;
        let half_size = (grid_size / 10) as i32 * 5;
        commands.spawn((
            meshes.add(Self::create_grid3d_mesh(10, 1.0)),
            SpatialBundle::default(),
            GridInstances(
                (0..(grid_size * grid_size))
                    .map(|i| {
                        let x = (i % grid_size) as i32 * 10 - half_size * 10 + 5;
                        let z = (i / grid_size) as i32 * 10 - half_size * 10 + 5;

                        GridInstanceData {
                            position: Vec3::new(x as f32, 0.0, z as f32),
                            scale: 1.0,
                        }
                    })
                    .collect(),
            ),
            NoFrustumCulling,
            Grid3d,
        ));
    }

    fn update(
        mut grids: Query<(&mut GridInstances, &Visibility), With<Grid3d>>,
        camera: Query<(&Projection, &PanOrbitState), With<PrimaryCamera>>,
    ) {
        let Ok((mut grid, visibility)) = grids.get_single_mut() else {
            return;
        };

        if visibility == Visibility::Hidden {
            return;
        }

        let (projection, orbit_state) = camera.single();

        let plane = Plane3d::new(Vec3::new(0.0, 1.0, 0.0));

        let frustum_corners = match projection {
            Projection::Perspective(projection) => {
                projection.get_frustum_corners(projection.near, projection.far)
            }
            Projection::Orthographic(projection) => {
                projection.get_frustum_corners(projection.near, projection.far)
            }
        };

        let distance = orbit_state.radius;

        let (base_size, scale) = if distance > 12000.0 {
            (10000, 1000.0)
        } else if distance > 800.0 {
            (1000, 100.0)
        } else if distance > 100.0 {
            (100, 10.0)
        } else {
            (10, 1.0)
        };

        let grid_size = 30;

        let half_size = (grid_size * base_size) as i32 / 2;

        for (i, instance) in grid.iter_mut().enumerate() {
            let x = (i % grid_size as usize) as i32 * base_size - half_size + base_size / 2;
            let z = (i / grid_size as usize) as i32 * base_size - half_size + base_size / 2;

            instance.position = Vec3::new(x as f32, 0.0, z as f32);
            instance.scale = scale;
        }
    }

    fn create_grid3d_mesh(size: u32, spacing: f32) -> Mesh {
        let half_size = (size as f32 * 0.5) as i32;
        let mut positions = Vec::new();
        let mut indices = Vec::new();
        let mut colors = Vec::new();

        let mut mesh = Mesh::new(
            PrimitiveTopology::LineList,
            RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD,
        );

        let mut index: u16 = 0;

        let line_color = [0.1, 0.1, 0.1, 1.0f32];
        let line_edge_color = [0.12, 0.12, 0.12, 1.0f32];

        for i in -half_size..=half_size {
            let x = i as f32 * spacing;
            positions.push(Vec3::new(x, 0.0, -half_size as f32 * spacing));
            positions.push(Vec3::new(x, 0.0, half_size as f32 * spacing));
            indices.push(index);
            indices.push(index + 1);

            if i == -half_size || i == half_size {
                colors.push(line_edge_color);
                colors.push(line_edge_color);
            } else {
                colors.push(line_color.clone());
                colors.push(line_color.clone());
            }
            index += 2
        }

        for i in -half_size..=half_size {
            let z = i as f32 * spacing;
            positions.push(Vec3::new(-half_size as f32 * spacing, 0.0, z));
            positions.push(Vec3::new(half_size as f32 * spacing, 0.0, z));
            indices.push(index);
            indices.push(index + 1);

            if i == -half_size || i == half_size {
                colors.push(line_edge_color);
                colors.push(line_edge_color);
            } else {
                colors.push(line_color);
                colors.push(line_color);
            }

            index += 2
        }
        let normals = vec![[0.0, 1.0, 0.0f32]; positions.len()];
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
        mesh.insert_indices(Indices::U16(indices));
        mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
        return mesh;
    }
}

#[derive(Resource)]
struct GridPipeline {
    shader: Handle<Shader>,
    mesh_pipeline: MeshPipeline,
    bind_group_layout: BindGroupLayout,
    uniform: UniformBuffer<GridData>,
    bind_group: BindGroup,
}

#[derive(Component)]
struct GridUniformBindGroup {
    bind_group: BindGroup,
}

impl FromWorld for GridPipeline {
    fn from_world(world: &mut World) -> Self {
        let asset_server = world.resource::<AssetServer>();
        let shader = asset_server.add(Shader::from_wgsl(include_str!("grid.wgsl"), "grid.wgsl"));

        let mesh_pipeline = world.resource::<MeshPipeline>();
        let render_device = world.resource::<RenderDevice>();
        let render_queue = world.resource::<RenderQueue>();

        let mut uniform = UniformBuffer::from(GridData::default());

        let bind_group_layout = render_device.create_bind_group_layout(
            "Grid Uniform Bind Group Layout",
            &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::VERTEX,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        );

        uniform.write_buffer(&*render_device, &*render_queue);

        let bind_group = render_device.create_bind_group(
            "Grid Uniform Bind Group",
            &bind_group_layout,
            &[BindGroupEntry {
                binding: 0,
                resource: uniform.binding().unwrap(),
            }],
        );

        GridPipeline {
            bind_group,
            shader,
            uniform,
            mesh_pipeline: mesh_pipeline.clone(),
            bind_group_layout,
        }
    }
}

impl SpecializedMeshPipeline for GridPipeline {
    type Key = MeshPipelineKey;

    fn specialize(
        &self,
        key: Self::Key,
        layout: &bevy::render::mesh::MeshVertexBufferLayout,
    ) -> Result<
        bevy::render::render_resource::RenderPipelineDescriptor,
        bevy::render::render_resource::SpecializedMeshPipelineError,
    > {
        let mut descriptor = self.mesh_pipeline.specialize(key, layout)?;
        descriptor.vertex.shader = self.shader.clone();
        descriptor.vertex.buffers.push(VertexBufferLayout {
            array_stride: std::mem::size_of::<GridInstanceData>() as u64,
            step_mode: VertexStepMode::Instance,
            attributes: vec![VertexAttribute {
                format: VertexFormat::Float32x4,
                offset: 0,
                shader_location: 3, // shader locations 0-2 are taken up by Position, Normal and UV attributes
            }],
        });
        descriptor.fragment.as_mut().unwrap().shader = self.shader.clone();

        descriptor.layout.push(self.bind_group_layout.clone());
        Ok(descriptor)
    }
}

type DrawGrid = (
    SetItemPipeline,
    SetMeshViewBindGroup<0>,
    SetMeshBindGroup<1>,
    SetGridUniforms<2>,
    DrawMeshInstanced,
);

struct SetGridUniforms<const I: usize>;

impl<P: PhaseItem, const I: usize> RenderCommand<P> for SetGridUniforms<I> {
    type Param = ();
    type ViewQuery = ();
    type ItemQuery = Read<GridUniformBindGroup>;

    fn render<'w>(
        _item: &P,
        _view: bevy::ecs::query::ROQueryItem<'w, Self::ViewQuery>,
        uniform: Option<bevy::ecs::query::ROQueryItem<'w, Self::ItemQuery>>,
        _param: bevy::ecs::system::SystemParamItem<'w, '_, Self::Param>,
        pass: &mut bevy::render::render_phase::TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let Some(bind_group) = uniform else {
            return RenderCommandResult::Failure;
        };

        pass.set_bind_group(I, &bind_group.bind_group, &[]);
        RenderCommandResult::Success
    }
}

struct DrawMeshInstanced;

impl<P: PhaseItem> RenderCommand<P> for DrawMeshInstanced {
    type Param = (SRes<RenderAssets<Mesh>>, SRes<RenderMeshInstances>);
    type ViewQuery = ();
    type ItemQuery = Read<InstanceBuffer>;

    #[inline]
    fn render<'w>(
        item: &P,
        _view: bevy::ecs::query::ROQueryItem<'w, Self::ViewQuery>,
        instance_buffer: Option<bevy::ecs::query::ROQueryItem<'w, Self::ItemQuery>>,
        (meshes, render_mesh_instances): bevy::ecs::system::SystemParamItem<'w, '_, Self::Param>,
        pass: &mut bevy::render::render_phase::TrackedRenderPass<'w>,
    ) -> bevy::render::render_phase::RenderCommandResult {
        let Some(mesh_instance) = render_mesh_instances.get(&item.entity()) else {
            return RenderCommandResult::Failure;
        };
        let Some(gpu_mesh) = meshes.into_inner().get(mesh_instance.mesh_asset_id) else {
            return RenderCommandResult::Failure;
        };
        let Some(instance_buffer) = instance_buffer else {
            return RenderCommandResult::Failure;
        };

        pass.set_vertex_buffer(0, gpu_mesh.vertex_buffer.slice(..));
        pass.set_vertex_buffer(1, instance_buffer.buffer.slice(..));

        match &gpu_mesh.buffer_info {
            GpuBufferInfo::Indexed {
                buffer,
                index_format,
                count,
            } => {
                pass.set_index_buffer(buffer.slice(..), 0, *index_format);
                pass.draw_indexed(0..*count, 0, 0..instance_buffer.length as u32);
            }
            GpuBufferInfo::NonIndexed => {
                pass.draw(0..gpu_mesh.vertex_count, 0..instance_buffer.length as u32);
            }
        }
        RenderCommandResult::Success
    }
}
