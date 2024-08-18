use bevy::{
    core_pipeline::core_3d::Transparent3d,
    ecs::{
        query::QueryItem,
        system::lifetimeless::{Read, SRes},
    },
    math::{
        bounding::{Aabb3d, Bounded3d, BoundingVolume},
        Vec3A,
    },
    pbr::{
        MeshPipeline, MeshPipelineKey, RenderMeshInstances, SetMeshBindGroup, SetMeshViewBindGroup,
    },
    prelude::*,
    render::{
        camera::CameraProjection,
        extract_component::{ExtractComponent, ExtractComponentPlugin},
        mesh::{GpuBufferInfo, Indices, PrimitiveTopology},
        render_asset::{RenderAssetUsages, RenderAssets},
        render_phase::{
            AddRenderCommand, DrawFunctions, PhaseItem, RenderCommand, RenderCommandResult,
            RenderPhase, SetItemPipeline,
        },
        render_resource::{
            AsBindGroup, BindGroup, BindGroupEntry, BindGroupLayout, BindGroupLayoutEntry,
            BindingType, Buffer, BufferBindingType, BufferInitDescriptor, BufferUsages,
            PipelineCache, ShaderStages, ShaderType, SpecializedMeshPipeline,
            SpecializedMeshPipelines, UniformBuffer, VertexAttribute, VertexBufferLayout,
            VertexFormat, VertexStepMode,
        },
        renderer::{RenderDevice, RenderQueue},
        view::{ExtractedView, NoFrustumCulling},
        Render, RenderApp, RenderSet,
    },
};
use bytemuck::{Pod, Zeroable};

use crate::utils;

use super::pan_orbit_camera::{PanOrbitCameraUpdate, PanOrbitState, PrimaryCamera};

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
    type QueryFilter = With<Self>;
    type Out = Self;

    fn extract_component(_item: QueryItem<'_, Self::QueryData>) -> Option<Self> {
        Some(PrimaryCamera)
    }
}

impl Plugin for GridPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(ExtractComponentPlugin::<GridInstances>::default())
            .add_plugins(ExtractComponentPlugin::<PrimaryCamera>::default())
            .add_systems(Update, update_grid.after(PanOrbitCameraUpdate))
            .add_systems(Startup, setup_grid)
            .sub_app_mut(RenderApp)
            .add_render_command::<Transparent3d, DrawGrid>()
            .init_resource::<SpecializedMeshPipelines<GridPipeline>>()
            .add_systems(
                Render,
                (
                    queue_grid.in_set(RenderSet::QueueMeshes),
                    prepare_instance_buffers.in_set(RenderSet::PrepareResources),
                ),
            );
    }

    fn finish(&self, app: &mut App) {
        app.sub_app_mut(RenderApp).init_resource::<GridPipeline>();
    }
}

#[derive(AsBindGroup, ShaderType, Default, Zeroable, Pod, Copy, Clone)]
#[repr(C)]
struct GridUniform {
    _padding: UVec3,
    #[uniform(0)]
    model_index: u32,
}

fn prepare_instance_buffers(
    mut commands: Commands,
    query: Query<(Entity, Ref<GridInstances>)>,
    render_device: Res<RenderDevice>,
) {
    for (entity, grid_instance_data) in &query {
        let buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
            label: Some("Grid Instance Data Buffer"),
            contents: bytemuck::cast_slice(grid_instance_data.as_slice()),
            usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
        });

        commands.entity(entity).insert((InstanceBuffer {
            buffer,
            length: grid_instance_data.len(),
        },));
    }
}

fn queue_grid(
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

    for (view, mut transparent_phase, _) in &mut views {
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

fn setup_grid(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>) {
    commands.spawn((
        meshes.add(create_grid_base(10, 1.0)),
        SpatialBundle::default(),
        GridInstances::default(),
        NoFrustumCulling,
        Grid3d,
    ));
}

fn frustum_corners_to_lines(corners: &[Vec3A; 8]) -> [(Vec3A, Vec3A); 12] {
    [
        // Near plane edges
        (corners[0], corners[1]), // bottom right to top right
        (corners[1], corners[2]), // top right to top left
        (corners[2], corners[3]), // top left to bottom left
        (corners[3], corners[0]), // bottom left to bottom right
        // Far plane edges
        (corners[4], corners[5]), // bottom right to top right
        (corners[5], corners[6]), // top right to top left
        (corners[6], corners[7]), // top left to bottom left
        (corners[7], corners[4]), // bottom left to bottom right
        // Connecting edges between near and far planes
        (corners[0], corners[4]), // bottom right near to far
        (corners[1], corners[5]), // top right near to far
        (corners[2], corners[6]), // top left near to far
        (corners[3], corners[7]), // bottom left near to far
    ]
}

fn line_intersects_plane_at(line: &(Vec3, Vec3), plane: &Plane3d) -> Option<Vec3> {
    let origin = line.0;
    let range = line.1 - line.0;
    let direction = range.normalize();
    let max = range.length();

    let ray = Ray3d::new(origin, direction);

    match ray.intersect_plane(Vec3::ZERO, plane.clone()) {
        Some(t) => {
            if t < max {
                Some(ray.get_point(t))
            } else {
                None
            }
        }
        None => None,
    }
}

fn update_grid(
    mut grid: Query<(&mut GridInstances, &Visibility), With<Grid3d>>,
    camera: Query<(&Projection, &PanOrbitState, Ref<Transform>), With<PrimaryCamera>>,
) {
    let Ok((mut grid, visibility)) = grid.get_single_mut() else {
        return;
    };

    if visibility == Visibility::Hidden {
        return;
    }

    let (projection, orbit_state, camera_transform) = camera.single();

    if !(camera_transform.is_added() || camera_transform.is_changed()) {
        return;
    }

    let camera_orientation = Quat::from_rotation_arc(Vec3::Z, camera_transform.forward().into());

    let plane = Plane3d::new(Direction3d::Y.into());

    let frustum_corners = match projection {
        Projection::Perspective(projection) => projection.get_frustum_corners(
            projection.near,
            plane.normal.dot(camera_transform.forward().into()).abs() * projection.far,
        ),
        Projection::Orthographic(projection) => {
            projection.get_frustum_corners(projection.near, projection.far)
        }
    };

    let lines = frustum_corners_to_lines(&frustum_corners);

    let transform_point = |point: Vec3A| -> Vec3 {
        let point = camera_orientation * Vec3::from(point) + camera_transform.translation;
        point
    };

    let aabb = lines
        .iter()
        .map(|line| (transform_point(line.0), transform_point(line.1)))
        .fold(None, |bounds, line| {
            let intersection = line_intersects_plane_at(&line, &plane);

            match intersection {
                Some(intersection) => match bounds {
                    Some((min, max)) => {
                        let min = intersection.min(min);
                        let max = intersection.max(max);
                        Some((min, max))
                    }
                    None => Some((intersection, intersection)),
                },
                None => bounds,
            }
        })
        .map_or(None, |(min, max)| Some(Aabb3d { min, max }));

    grid.clear();

    let Some(mut aabb) = aabb else {
        info!("Plane is not visible");
        return;
    };

    let area = aabb.visible_area();
    info!("{}", area * 0.5);

    let distance = orbit_state.radius;

    let size = if distance > 12000.0 {
        10000.0f32
    } else if distance > 800.0 {
        1000.0f32
    } else if distance > 200.0 {
        100.0f32
    } else {
        10.0f32
    };

    let half_size = size * 0.5;
    let padding = 10.0;

    aabb.min -= padding;
    aabb.max += padding;

    let compute_starting_min = |value: Vec3| -> Vec3 {
        Vec3::new(
            utils::multiples::largest_multiple_less_than_or_equal_to(value.x, size),
            0.0,
            utils::multiples::largest_multiple_less_than_or_equal_to(value.z, size),
        )
    };

    let compute_starting_max = |value: Vec3| -> Vec3 {
        Vec3::new(
            utils::multiples::smallest_multiple_greater_than_or_equal_to(value.x, size),
            0.0,
            utils::multiples::smallest_multiple_greater_than_or_equal_to(value.z, size),
        )
    };

    // let (min, max) = (
    //     compute_starting_min(aabb.min),
    //     compute_starting_max(aabb.max),
    // );

    // for x in (min.x as i32..=max.x as i32).step_by(size as usize) {
    //     for z in (min.z as i32..=max.z as i32).step_by(size as usize) {
    //         grid.push(GridInstanceData {
    //             position: Vec3::new(x as f32, 0.0, z as f32),
    //             scale: size / 10.0,
    //         });
    //     }
    // }

    info!("Grid size: {}", grid.len());

    let distance = orbit_state.radius;

    let (base_size, scale) = if distance > 12000.0 {
        (10000, 1000.0)
    } else if distance > 800.0 {
        (1000, 100.0)
    } else if distance > 200.0 {
        (100, 10.0)
    } else {
        (10, 1.0)
    };

    let grid_size = 30;
    let half_size = (grid_size * base_size) as i32 / 2;

    for i in 0..grid_size * grid_size {
        let x = (i as usize % grid_size as usize) as i32 * base_size - half_size + base_size / 2;
        let z = (i as usize / grid_size as usize) as i32 * base_size - half_size + base_size / 2;

        grid.push(GridInstanceData {
            position: Vec3::new(x as f32, 0.0, z as f32),
            scale,
        });
    }
}

fn create_grid_base(size: u32, spacing: f32) -> Mesh {
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

#[derive(Resource)]
struct GridPipeline {
    shader: Handle<Shader>,
    mesh_pipeline: MeshPipeline,
    bind_group_layout: BindGroupLayout,
    uniform: UniformBuffer<GridUniform>,
    bind_group: BindGroup,
}

impl FromWorld for GridPipeline {
    fn from_world(world: &mut World) -> Self {
        let asset_server = world.resource::<AssetServer>();
        let shader = asset_server.add(Shader::from_wgsl(include_str!("grid.wgsl"), "grid.wgsl"));

        let mesh_pipeline = world.resource::<MeshPipeline>();
        let render_device = world.resource::<RenderDevice>();
        let render_queue = world.resource::<RenderQueue>();

        let mut uniform = UniformBuffer::from(GridUniform::default());

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
                shader_location: 3,
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
    SetGridUniformBindGroup<2>,
    DrawMeshInstanced,
);

struct SetGridUniformBindGroup<const I: usize>;

impl<P: PhaseItem, const I: usize> RenderCommand<P> for SetGridUniformBindGroup<I> {
    type Param = (SRes<RenderQueue>, SRes<GridPipeline>);
    type ViewQuery = ();
    type ItemQuery = ();

    fn render<'w>(
        item: &P,
        _view: bevy::ecs::query::ROQueryItem<'w, Self::ViewQuery>,
        _query: Option<bevy::ecs::query::ROQueryItem<'w, Self::ItemQuery>>,
        (render_queue, grid_pipeline): bevy::ecs::system::SystemParamItem<'w, '_, Self::Param>,
        pass: &mut bevy::render::render_phase::TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let model_index = item.batch_range().clone().next().unwrap();
        render_queue.write_buffer(
            grid_pipeline.uniform.buffer().unwrap(),
            0,
            bytemuck::cast_slice(&[GridUniform {
                model_index,
                ..Default::default()
            }]),
        );
        let grid_pipeline = grid_pipeline.into_inner();
        pass.set_bind_group(I, &grid_pipeline.bind_group, &[]);
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
