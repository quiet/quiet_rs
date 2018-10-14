extern crate fec;
extern crate libc;

use fec::convolutional::{Decoder, Encoder};
use libc::{size_t, ssize_t};
use std::slice;

#[repr(C)]
pub struct Convolutional {
    encoder: Encoder,
    decoder: Decoder,
}

#[no_mangle]
pub extern "C" fn correct_convolutional_create(
    rate: size_t,
    order: size_t,
    c_polys: *const u16,
) -> *mut Convolutional {
    let polys: &[u16];
    unsafe {
        polys = slice::from_raw_parts(c_polys, rate);
    }
    let conv = Box::new(Convolutional {
        encoder: Encoder::new(rate as u32, order as u32, polys),
        decoder: Decoder::new(rate as u32, order as u32, polys),
    });
    Box::into_raw(conv)
}

#[no_mangle]
pub extern "C" fn correct_convolutional_destroy(conv_ptr: *mut Convolutional) {
    unsafe {
        Box::from_raw(conv_ptr);
    }
}

#[no_mangle]
pub extern "C" fn correct_convolutional_encode_len(
    conv_ptr: *const Convolutional,
    msg_len: size_t,
) -> size_t {
    let conv: &Convolutional;
    unsafe {
        conv = &*conv_ptr;
    }
    conv.encoder.encode_len(msg_len)
}

#[no_mangle]
pub extern "C" fn correct_convolutional_encode(
    conv_ptr: *mut Convolutional,
    msg_ptr: *const u8,
    msg_len: size_t,
    encoded_ptr: *mut u8,
) -> size_t {
    let conv: &mut Convolutional;
    let msg: &[u8];
    let encoded: &mut [u8];
    unsafe {
        conv = &mut *conv_ptr;
        msg = slice::from_raw_parts(msg_ptr, msg_len);
        let encode_len = conv.encoder.encode_len(msg_len) / 8 + 1;
        encoded = slice::from_raw_parts_mut(encoded_ptr, encode_len);
    }
    conv.encoder.encode(msg, encoded)
}

#[no_mangle]
pub extern "C" fn correct_convolutional_decode(
    conv_ptr: *mut Convolutional,
    encoded_ptr: *const u8,
    num_encoded_bits: size_t,
    msg_ptr: *mut u8,
) -> ssize_t {
    let conv: &mut Convolutional;
    let encoded: &[u8];
    let msg: &mut [u8];
    unsafe {
        conv = &mut *conv_ptr;
        let encoded_len = num_encoded_bits / 8 + 1;
        encoded = slice::from_raw_parts(encoded_ptr, encoded_len);
        msg = slice::from_raw_parts_mut(msg_ptr, encoded_len);
    }
    conv.decoder.decode(encoded, num_encoded_bits, msg)
}

#[no_mangle]
pub extern "C" fn correct_convolutional_decode_soft(
    conv_ptr: *mut Convolutional,
    soft: *const u8,
    num_encoded_bits: size_t,
    msg: *mut u8,
) -> ssize_t {
    0
}
