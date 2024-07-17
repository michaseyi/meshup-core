use bevy::{
    prelude::*,
    render::{
        mesh::{Indices, PrimitiveTopology},
        render_asset::RenderAssetUsages,
    },
};

pub struct GridPlugin;

impl Plugin for GridPlugin {
    fn build(&self, app: &mut App) {
        // app.add_startup_system(setup.system());
    }
}

#[derive(Bundle, Default)]
pub struct Grid3dBundle {
    mesh: PbrBundle,
}
impl GridPlugin {
    pub fn create_grid3d_mesh(size: u32, spacing: f32) -> Mesh {
        let half_size = (size as f32 * 0.5) as i32;
        let mut positions = Vec::new();
        let mut indices = Vec::new();
        let mut colors = Vec::new();

        let mut mesh = Mesh::new(PrimitiveTopology::LineList, RenderAssetUsages::RENDER_WORLD);

        let mut index: u16 = 0;

        let line_color = [0.55, 0.55, 0.55, 1.0f32];
        let line_edge_color = [0.75, 0.75, 0.75, 1.0f32];

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
        // mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
        return mesh;
    }
}
