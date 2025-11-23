#![no_std]

use grid::GridRefMut;
use push_constants::shader::*;
use shared::*;
use spirv_std::glam::*;
use spirv_std::spirv;

#[spirv(fragment)]
pub fn main_fs(
    #[spirv(frag_coord)] frag_coord: Vec4,
    #[cfg(not(feature = "emulate_constants"))]
    #[spirv(push_constant)]
    constants: &FragmentConstants,
    #[cfg(feature = "emulate_constants")]
    #[spirv(storage_buffer, descriptor_set = 1, binding = 0)]
    constants: &FragmentConstants,
    #[spirv(storage_buffer, descriptor_set = 0, binding = 0)] cell_grid: &mut [CellState],
    output: &mut Vec4,
) {
    let mut cell_grid = GridRefMut::new(DIM, cell_grid);
    let coord = frag_coord.xy() - constants.translate;
    let i = ((coord / constants.size.as_vec2() / constants.camera_zoom
        + constants.camera_translate)
        * DIM.as_vec2())
    .as_uvec2();

    if constants.mouse_button_pressed & 1 == 1 && constants.cursor.distance_squared(coord) < 0.5 {
        cell_grid.set(i, CellState::On);
    }

    let col = match cell_grid.get(i) {
        CellState::Off => Vec3::ZERO,
        CellState::On => Vec3::X,
        CellState::Dying => vec3(0.3, 0.05, 0.0),
        CellState::Spawning => vec3(0.35, 0.0, 0.0),
    };
    *output = col.extend(1.0);
}

#[spirv(vertex)]
pub fn main_vs(
    #[spirv(vertex_index)] vert_id: i32,
    #[spirv(position, invariant)] out_pos: &mut Vec4,
) {
    let uv = vec2(((vert_id << 1) & 2) as f32, (vert_id & 2) as f32);
    let pos = 2.0 * uv - Vec2::ONE;
    *out_pos = pos.extend(0.0).extend(1.0);
}

#[spirv(compute(threads(16, 16)))]
pub fn main_cs(
    #[spirv(global_invocation_id)] gid: UVec3,
    #[cfg(not(feature = "emulate_constants"))]
    #[spirv(push_constant)]
    constants: &ComputeConstants,
    #[cfg(feature = "emulate_constants")]
    #[spirv(storage_buffer, descriptor_set = 1, binding = 1)]
    constants: &ComputeConstants,
    #[spirv(storage_buffer, descriptor_set = 0, binding = 0)] cell_grid: &mut [CellState],
) {
    let mut cell_grid = GridRefMut::new(DIM, cell_grid);
    let index = gid.xy();
    let val = cell_grid.get(index);

    if constants.transition.into() {
        cell_grid.set(
            index,
            match val {
                CellState::Dying => CellState::Off,
                CellState::Spawning => CellState::On,
                CellState::Off => CellState::Off,
                CellState::On => CellState::On,
            },
        );
        return;
    }

    let mut count = 0;
    for i in -1..=1 {
        for j in -1..=1 {
            let ij = ivec2(i, j);
            if ij == IVec2::ZERO {
                continue;
            }
            let index = (index.as_ivec2() + ij)
                .rem_euclid(DIM.as_ivec2())
                .as_uvec2();

            let val = cell_grid.get(index);
            if matches!(val, CellState::On | CellState::Dying) {
                count += 1
            }
        }
    }

    // Clippy would prefer that we wrote this as  `!(2..=3).contains(&count)`, but that causes the spirv-builder compiler to fail
    #[expect(clippy::manual_range_contains, reason="work around spirv-builder compiler failure")]
    if matches!(val, CellState::On) && (count < 2 || count > 3) {
        cell_grid.set(index, CellState::Dying);
    } else if matches!(val, CellState::Off) && count == 3 {
        cell_grid.set(index, CellState::Spawning);
    }
}
