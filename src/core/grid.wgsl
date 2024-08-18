#import bevy_pbr::mesh_functions::{get_model_matrix, mesh_position_local_to_clip}
#import bevy_pbr::{
    mesh_view_bindings::view,
}

struct Vertex {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(5) color: vec4<f32>,
    @builtin(instance_index) instance_index: u32,
    @location(3) i_pos_scale: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) position: vec3<f32>,
};

struct GridData {
    _padding: vec3<u32>,
    model_index: u32,
}

@group(2) @binding(0) var<uniform> grid_data: GridData;


@vertex
fn vertex(vertex: Vertex) -> VertexOutput {
    let position = vertex.position * vertex.i_pos_scale.w + vertex.i_pos_scale.xyz;
    var out: VertexOutput;
    out.clip_position = mesh_position_local_to_clip(
        get_model_matrix(grid_data.model_index),
        vec4<f32>(position, 1.0)
    );
    out.position = position.xyz;
    out.color = vertex.color;
    return out;
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let camera_vector = view.world_position - in.position;
    let len = length(camera_vector);
    let camera_vector_n = normalize(camera_vector);
    let alpha = pow(abs(dot(camera_vector_n, vec3<f32>(0.0, 1.0, 0.0))), 1.0) * min(pow((50.0 / len), 2.0), 1.0);
    return vec4<f32>(in.color.xyz, 1.0);
}
