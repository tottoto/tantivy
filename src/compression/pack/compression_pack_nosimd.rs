use common::bitpacker::compute_num_bits;
use common::bitpacker::{BitPacker, BitUnpacker};
use std::cmp;
use std::io::Write;
use super::super::NUM_DOCS_PER_BLOCK;

const COMPRESSED_BLOCK_MAX_SIZE: usize = NUM_DOCS_PER_BLOCK * 4 + 1;

pub fn compress_sorted(vals: &mut [u32], mut output: &mut [u8], offset: u32) -> usize {
    let mut max_delta = 0;
    {
        let mut local_offset = offset;
        for i in 0..NUM_DOCS_PER_BLOCK {
            let val = vals[i];
            let delta = val - local_offset;
            max_delta = cmp::max(max_delta, delta);
            vals[i] = delta;
            local_offset = val;
        }
    }
    let num_bits = compute_num_bits(max_delta as u64);
    output.write_all(&[num_bits]).unwrap();
    let mut bit_packer = BitPacker::new(num_bits as usize);
    for val in vals {
        bit_packer.write(*val, &mut output).unwrap();
    }
    1 +
    bit_packer
        .close(&mut output)
        .expect("packing in memory should never fail")
}



pub struct BlockEncoder {
    pub output: [u8; COMPRESSED_BLOCK_MAX_SIZE],
    pub output_len: usize,
    input_buffer: [u32; NUM_DOCS_PER_BLOCK],
}

impl BlockEncoder {
    pub fn new() -> BlockEncoder {
        BlockEncoder {
            output: [0u8; COMPRESSED_BLOCK_MAX_SIZE],
            output_len: 0,
            input_buffer: [0u32; NUM_DOCS_PER_BLOCK],
        }
    }

    pub fn compress_block_sorted(&mut self, vals: &[u32], offset: u32) -> &[u8] {
        self.input_buffer.clone_from_slice(vals);
        let compressed_size = compress_sorted(&mut self.input_buffer, &mut self.output, offset);
        &self.output[..compressed_size]
    }

    pub fn compress_block_unsorted(&mut self, vals: &[u32]) -> &[u8] {
        let compressed_size: usize = {
            let mut output: &mut [u8] = &mut self.output;
            let max = vals.iter()
                .cloned()
                .max()
                .expect("compress unsorted called with an empty array");
            let num_bits = compute_num_bits(max);
            output.write_all(&[num_bits]).unwrap();
            let mut bit_packer = BitPacker::new(num_bits as usize);
            for val in vals {
                bit_packer.write(*val, &mut output).unwrap();
            }
            1 +
            bit_packer
                .close(&mut output)
                .expect("packing in memory should never fail")
        };
        &self.output[..compressed_size]
    }
}

pub struct BlockDecoder {
    pub output: [u32; COMPRESSED_BLOCK_MAX_SIZE],
    pub output_len: usize,
}


impl BlockDecoder {
    pub fn new() -> BlockDecoder {
        BlockDecoder::with_val(0u32)
    }

    pub fn with_val(val: u32) -> BlockDecoder {
        BlockDecoder {
            output: [val; COMPRESSED_BLOCK_MAX_SIZE],
            output_len: 0,
        }
    }

    pub fn uncompress_block_sorted<'a>(&mut self,
                                       compressed_data: &'a [u8],
                                       mut offset: u32)
                                       -> &'a [u8] {
        let consumed_size = {
            let num_bits = compressed_data[0];
            let bit_unpacker = BitUnpacker::new(&compressed_data[1..], num_bits as usize);
            for i in 0..NUM_DOCS_PER_BLOCK {
                let delta = bit_unpacker.get(i);
                let val = offset + delta;
                self.output[i] = val;
                offset = val;
            }
            1 + (num_bits as usize * NUM_DOCS_PER_BLOCK + 7) / 8
        };
        self.output_len = NUM_DOCS_PER_BLOCK;
        &compressed_data[consumed_size..]
    }

    pub fn uncompress_block_unsorted<'a>(&mut self, compressed_data: &'a [u8]) -> &'a [u8] {
        let num_bits = compressed_data[0];
        let bit_unpacker = BitUnpacker::new(&compressed_data[1..], num_bits as usize);
        for i in 0..NUM_DOCS_PER_BLOCK {
            self.output[i] = bit_unpacker.get(i);
        }
        let consumed_size = 1 + (num_bits as usize * NUM_DOCS_PER_BLOCK + 7) / 8;
        self.output_len = NUM_DOCS_PER_BLOCK;
        &compressed_data[consumed_size..]
    }

    #[inline]
    pub fn output_array(&self) -> &[u32] {
        &self.output[..self.output_len]
    }

    #[inline]
    pub fn output(&self, idx: usize) -> u32 {
        self.output[idx]
    }
}
