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
    // NOTE: Passing 0 as the instance_index to get_world_from_local() is a hack
    // for this example as the instance_index builtin would map to the wrong
    // index in the Mesh array. This index could be passed in via another
    // uniform instead but it's unnecessary for the example.
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
    let l = length(camera_vector);
    let camera_vector_n = normalize(camera_vector);
    let alpha = pow(abs(dot(camera_vector_n, vec3<f32>(0.0, 1.0, 0.0))), 1.0) * min(pow((50.0 / l), 2.0), 1.0);
    return vec4<f32>(in.color.xyz, alpha);
}
