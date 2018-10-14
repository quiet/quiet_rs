extern crate fec;
extern crate libc;

use fec::convolutional::Decoder;
use libc::{c_int, c_uint};
use std::iter;
use std::slice;

#[repr(C)]
pub struct Shim {
    decoder: Decoder,
    rate: u32,
    order: u32,
    decode_buffer: Vec<u8>,
    read_index: usize,
    write_index: usize,
}

impl Shim {
    fn new(num_decoded_bits: usize, rate: u32, order: u32, polys: &[u16]) -> Shim {
        let num_decoded: usize;
        if num_decoded_bits % 8 == 0 {
            num_decoded = num_decoded_bits / 8;
        } else {
            num_decoded = num_decoded_bits / 8 + 1;
        }

        let decode_buffer = vec![0; num_decoded + 1];

        Shim {
            decoder: Decoder::new(rate, order, polys),
            rate: rate,
            order: order,
            decode_buffer: decode_buffer,
            read_index: 0,
            write_index: 0,
        }
    }

    fn init(&mut self) {
        self.read_index = 0;
        self.write_index = 0;
    }

    fn decode(&mut self, encoded: &[u8]) {
        let remaining_buffer = self.decode_buffer.len() - self.write_index;
        let remaining_bits = 8 * remaining_buffer;
        let mut encoded_bits = encoded.len();

        let mut decoded_len = (encoded.len() / self.rate as usize) - (self.order as usize - 1);
        if decoded_len > remaining_bits {
            let over = decoded_len - remaining_bits;
            decoded_len -= over;
            encoded_bits -= over * self.rate as usize;
        }

        // XXX we need soft!
        let mut hard = vec![0; encoded_bits / 8 + 1];
        let extra_bits: usize;
        if encoded_bits % 8 == 0 {
            extra_bits = 0;
        } else {
            extra_bits = 8 - (encoded_bits % 8);
        }
        let mut soft_iter = encoded.iter().chain(iter::repeat(&0).take(extra_bits));
        for byte in hard.iter_mut().take(encoded_bits / 8) {
            *byte = (soft_iter.next().unwrap() & 1) << 7;
            *byte |= (soft_iter.next().unwrap() & 1) << 6;
            *byte |= (soft_iter.next().unwrap() & 1) << 5;
            *byte |= (soft_iter.next().unwrap() & 1) << 4;
            *byte |= (soft_iter.next().unwrap() & 1) << 3;
            *byte |= (soft_iter.next().unwrap() & 1) << 2;
            *byte |= (soft_iter.next().unwrap() & 1) << 1;
            *byte |= (soft_iter.next().unwrap() & 1);
        }

        self.decoder.decode(
            &hard,
            encoded_bits,
            &mut self.decode_buffer[self.write_index..],
        );
        self.write_index += decoded_len / 8;
    }

    fn receive(&mut self, decoded: &mut [u8]) {
        let remaining_buffer = self.write_index - self.read_index;
        let remaining_bits = remaining_buffer * 8;

        let mut receive_bits = decoded.len() * 8;
        if receive_bits > remaining_bits {
            receive_bits = remaining_bits;
        }

        let receive_len: usize;
        if receive_bits % 8 == 0 {
            receive_len = receive_bits / 8;
        } else {
            receive_len = receive_bits / 8 + 1;
        }

        decoded[..receive_len].clone_from_slice(
            &self.decode_buffer[self.read_index..(self.read_index + receive_len)],
        );
        self.read_index += receive_len;
    }
}

#[no_mangle]
pub extern "C" fn create_viterbi27(num_decoded_bits: c_int) -> *mut Shim {
    let shim = Box::new(Shim::new(num_decoded_bits as usize, 2, 7, &[0o155, 0o117]));
    Box::into_raw(shim)
}

#[no_mangle]
pub extern "C" fn delete_viterbi27(shim_ptr: *mut Shim) {
    unsafe {
        Box::from_raw(shim_ptr);
    }
}

#[no_mangle]
pub extern "C" fn init_viterbi27(shim_ptr: *mut Shim, _: c_int) -> c_int {
    let shim: &mut Shim;
    unsafe {
        shim = &mut *shim_ptr;
    }
    shim.init();
    0
}

#[no_mangle]
pub extern "C" fn update_viterbi27_blk(
    shim_ptr: *mut Shim,
    encoded_ptr: *const u8,
    num_groups: c_int,
) -> c_int {
    let shim: &mut Shim;
    let encoded: &[u8];
    unsafe {
        shim = &mut *shim_ptr;
        encoded = slice::from_raw_parts(encoded_ptr, num_groups as usize * shim.rate as usize);
    }
    shim.decode(encoded);
    0
}

#[no_mangle]
pub extern "C" fn chainback_viterbi27(
    shim_ptr: *mut Shim,
    decoded_ptr: *mut u8,
    num_bits: c_uint,
    _: c_int,
) -> c_int {
    let shim: &mut Shim;
    let decoded: &mut [u8];
    unsafe {
        shim = &mut *shim_ptr;
        decoded = slice::from_raw_parts_mut(decoded_ptr, num_bits as usize);
    }
    shim.receive(decoded);
    0
}

#[no_mangle]
pub extern "C" fn create_viterbi29(num_decoded_bits: c_int) -> *mut Shim {
    let shim = Box::new(Shim::new(num_decoded_bits as usize, 2, 9, &[0o657, 0o435]));
    Box::into_raw(shim)
}

#[no_mangle]
pub extern "C" fn delete_viterbi29(shim_ptr: *mut Shim) {
    unsafe {
        Box::from_raw(shim_ptr);
    }
}

#[no_mangle]
pub extern "C" fn init_viterbi29(shim_ptr: *mut Shim, _: c_int) -> c_int {
    let shim: &mut Shim;
    unsafe {
        shim = &mut *shim_ptr;
    }
    shim.init();
    0
}

#[no_mangle]
pub extern "C" fn update_viterbi29_blk(
    shim_ptr: *mut Shim,
    encoded_ptr: *const u8,
    num_groups: c_int,
) -> c_int {
    let shim: &mut Shim;
    let encoded: &[u8];
    unsafe {
        shim = &mut *shim_ptr;
        encoded = slice::from_raw_parts(encoded_ptr, num_groups as usize * shim.rate as usize);
    }
    shim.decode(encoded);
    0
}

#[no_mangle]
pub extern "C" fn chainback_viterbi29(
    shim_ptr: *mut Shim,
    decoded_ptr: *mut u8,
    num_bits: c_uint,
    _: c_int,
) -> c_int {
    let shim: &mut Shim;
    let decoded: &mut [u8];
    unsafe {
        shim = &mut *shim_ptr;
        decoded = slice::from_raw_parts_mut(decoded_ptr, num_bits as usize);
    }
    shim.receive(decoded);
    0
}

#[no_mangle]
pub extern "C" fn create_viterbi39(num_decoded_bits: c_int) -> *mut Shim {
    let shim = Box::new(Shim::new(
        num_decoded_bits as usize,
        3,
        9,
        &[0o755, 0o633, 0o447],
    ));
    Box::into_raw(shim)
}

#[no_mangle]
pub extern "C" fn delete_viterbi39(shim_ptr: *mut Shim) {
    unsafe {
        Box::from_raw(shim_ptr);
    }
}

#[no_mangle]
pub extern "C" fn init_viterbi39(shim_ptr: *mut Shim, _: c_int) -> c_int {
    let shim: &mut Shim;
    unsafe {
        shim = &mut *shim_ptr;
    }
    shim.init();
    0
}

#[no_mangle]
pub extern "C" fn update_viterbi39_blk(
    shim_ptr: *mut Shim,
    encoded_ptr: *const u8,
    num_groups: c_int,
) -> c_int {
    let shim: &mut Shim;
    let encoded: &[u8];
    unsafe {
        shim = &mut *shim_ptr;
        encoded = slice::from_raw_parts(encoded_ptr, num_groups as usize * shim.rate as usize);
    }
    shim.decode(encoded);
    0
}

#[no_mangle]
pub extern "C" fn chainback_viterbi39(
    shim_ptr: *mut Shim,
    decoded_ptr: *mut u8,
    num_bits: c_uint,
    _: c_int,
) -> c_int {
    let shim: &mut Shim;
    let decoded: &mut [u8];
    unsafe {
        shim = &mut *shim_ptr;
        decoded = slice::from_raw_parts_mut(decoded_ptr, num_bits as usize);
    }
    shim.receive(decoded);
    0
}

#[no_mangle]
pub extern "C" fn create_viterbi615(num_decoded_bits: c_int) -> *mut Shim {
    let shim = Box::new(Shim::new(
        num_decoded_bits as usize,
        6,
        15,
        &[0o42631, 0o47245, 0o56507, 0o73363, 0o77267, 0o64537],
    ));
    Box::into_raw(shim)
}

#[no_mangle]
pub extern "C" fn delete_viterbi615(shim_ptr: *mut Shim) {
    unsafe {
        Box::from_raw(shim_ptr);
    }
}

#[no_mangle]
pub extern "C" fn init_viterbi615(shim_ptr: *mut Shim, _: c_int) -> c_int {
    let shim: &mut Shim;
    unsafe {
        shim = &mut *shim_ptr;
    }
    shim.init();
    0
}

#[no_mangle]
pub extern "C" fn update_viterbi615_blk(
    shim_ptr: *mut Shim,
    encoded_ptr: *const u8,
    num_groups: c_int,
) -> c_int {
    let shim: &mut Shim;
    let encoded: &[u8];
    unsafe {
        shim = &mut *shim_ptr;
        encoded = slice::from_raw_parts(encoded_ptr, num_groups as usize * shim.rate as usize);
    }
    shim.decode(encoded);
    0
}

#[no_mangle]
pub extern "C" fn chainback_viterbi615(
    shim_ptr: *mut Shim,
    decoded_ptr: *mut u8,
    num_bits: c_uint,
    _: c_int,
) -> c_int {
    let shim: &mut Shim;
    let decoded: &mut [u8];
    unsafe {
        shim = &mut *shim_ptr;
        decoded = slice::from_raw_parts_mut(decoded_ptr, num_bits as usize);
    }
    shim.receive(decoded);
    0
}
