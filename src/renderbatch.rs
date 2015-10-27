use device::{ProgramId, TextureId};
use fnv::FnvHasher;
use internal_types::{BatchId, DisplayItemKey, DrawListIndex};
use internal_types::{PackedVertex, Primitive};
use std::collections::HashMap;
use std::collections::hash_map::Entry::{Occupied, Vacant};
use std::collections::hash_state::DefaultState;

const MAX_MATRICES_PER_BATCH: usize = 32;

pub struct RenderBatch {
    pub batch_id: BatchId,
    pub sort_key: DisplayItemKey,
    pub program_id: ProgramId,
    pub color_texture_id: TextureId,
    pub mask_texture_id: TextureId,
    pub vertices: Vec<PackedVertex>,
    pub indices: Vec<u16>,
    pub matrix_map: HashMap<DrawListIndex, u8, DefaultState<FnvHasher>>,
}

impl RenderBatch {
    pub fn new(batch_id: BatchId,
           sort_key: DisplayItemKey,
           program_id: ProgramId,
           color_texture_id: TextureId,
           mask_texture_id: TextureId) -> RenderBatch {
        RenderBatch {
            sort_key: sort_key,
            batch_id: batch_id,
            program_id: program_id,
            color_texture_id: color_texture_id,
            mask_texture_id: mask_texture_id,
            vertices: Vec::new(),
            indices: Vec::new(),
            matrix_map: HashMap::with_hash_state(Default::default()),
        }
    }

    pub fn can_add_to_batch(&self,
                        color_texture_id: TextureId,
                        mask_texture_id: TextureId,
                        key: &DisplayItemKey,
                        program_id: ProgramId) -> bool {
        let matrix_ok = self.matrix_map.len() < MAX_MATRICES_PER_BATCH ||
                        self.matrix_map.contains_key(&key.draw_list_index);
        let program_ok = program_id == self.program_id;
        let color_texture_ok = color_texture_id == self.color_texture_id;
        let mask_texture_ok = mask_texture_id == self.mask_texture_id;
        let vertices_ok = self.vertices.len() < 65535;  // to ensure we can use u16 index buffers

        let batch_ok = matrix_ok &&
                       program_ok &&
                       color_texture_ok &&
                       mask_texture_ok &&
                       vertices_ok;

        if !batch_ok {
            //println!("break batch! matrix={} program={} color={} mask={} vertices={} [{:?} vs {:?}]",
            //         matrix_ok, program_ok, color_texture_ok, mask_texture_ok, vertices_ok, color_texture_id, self.color_texture_id);
        }

        batch_ok
    }

    pub fn add_draw_item(&mut self,
                     color_texture_id: TextureId,
                     mask_texture_id: TextureId,
                     primitive: Primitive,
                     vertices: &mut [PackedVertex],
                     key: &DisplayItemKey) {
        debug_assert!(color_texture_id == self.color_texture_id);
        debug_assert!(mask_texture_id == self.mask_texture_id);

        let next_matrix_index = self.matrix_map.len() as u8;
        let matrix_index = match self.matrix_map.entry(key.draw_list_index) {
            Vacant(entry) => *entry.insert(next_matrix_index),
            Occupied(entry) => *entry.get(),
        };
        debug_assert!(self.matrix_map.len() <= MAX_MATRICES_PER_BATCH);

        let index_offset = self.vertices.len();

        match primitive {
            Primitive::Rectangles | Primitive::Glyphs => {
                for i in (0..vertices.len()).step_by(4) {
                    let index_base = (index_offset + i) as u16;
                    self.indices.push(index_base + 0);
                    self.indices.push(index_base + 1);
                    self.indices.push(index_base + 2);
                    self.indices.push(index_base + 2);
                    self.indices.push(index_base + 3);
                    self.indices.push(index_base + 1);
                }
            }
            Primitive::Triangles => {
                for i in (0..vertices.len()).step_by(3) {
                    let index_base = (index_offset + i) as u16;
                    self.indices.push(index_base + 0);
                    self.indices.push(index_base + 1);
                    self.indices.push(index_base + 2);
                }
            }
            Primitive::TriangleFan => {
                for i in (1..vertices.len() - 1) {
                    self.indices.push(index_offset as u16);        // center vertex
                    self.indices.push((index_offset + i + 0) as u16);
                    self.indices.push((index_offset + i + 1) as u16);
                }
            }
        }

        for vertex in vertices.iter_mut() {
            vertex.matrix_index = matrix_index;
        }

        self.vertices.push_all(vertices);
    }
}