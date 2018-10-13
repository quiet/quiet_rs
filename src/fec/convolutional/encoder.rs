use super::bit::{BitReader, BitWriter};
use super::util;

#[derive(Debug)]
pub struct Encoder {
    rate: u32,
    order: u32,
    poly_table: Vec<u16>,
}

impl Encoder {
    pub fn new(rate: u32, order: u32, polys: &[u16]) -> Encoder {
        Encoder {
            rate: rate,
            order: order,
            poly_table: util::conv_poly_table(rate, order, polys),
        }
    }

    pub fn encode_len(&self, len: usize) -> usize {
        let bits = len * 8;
        self.rate as usize * (bits + self.order as usize + 1)
    }

    pub fn encode(&mut self, msg: &[u8], dst: &mut [u8]) -> usize {
        let mut bit_reader = BitReader::new(msg);
        let mut bit_writer = BitWriter::new(dst);

        let mut shift_register: u32 = 0;
        let shift_mask: u32 = (1 << self.order) - 1;
        let encode_len = self.encode_len(msg.len());

        for _i in 0..8 * msg.len() {
            shift_register <<= 1;
            shift_register |= bit_reader.read(1) as u32;
            shift_register &= shift_mask;

            bit_writer.write(
                self.poly_table[shift_register as usize] as u8,
                self.rate as usize,
            );
        }

        for _i in 0..self.order + 1 {
            shift_register <<= 1;
            shift_register &= shift_mask;
            bit_writer.write(
                self.poly_table[shift_register as usize] as u8,
                self.rate as usize,
            );
        }

        bit_writer.flush();
        encode_len
    }
}
