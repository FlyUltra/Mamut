use super::chunk::ChunkColumn;

pub const BLOCK_STONE: u16 = 1;

// Generate a simple visible test world:
//   y=0..15: stone
//   everything else: air
pub fn generate_flat_chunk(cx: i32, cz: i32) -> ChunkColumn {
    let mut col = ChunkColumn::new_empty(cx, cz);

    for bx in 0u8..16 {
        for bz in 0u8..16 {
            for y in 0..=15 {
                col.set_block(bx, y, bz, BLOCK_STONE);
            }
        }
    }

    col
}