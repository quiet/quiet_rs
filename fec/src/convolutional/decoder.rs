use super::bit::{BitReader, BitWriter};
use super::util;

use std::collections::HashMap;
use std::iter::Iterator;
use std::mem;

#[derive(Debug)]
pub struct Decoder {
    rate: u32,
    order: u32,
    highbit: u16,
    poly_table: Vec<u16>,
    pair_table: ConvolutionalPairTable,
    history_table: ConvolutionalHistoryTable,
    error_table: ConvolutionalErrorTable,
    distances: Vec<u16>,
}

impl Decoder {
    pub fn new(rate: u32, order: u32, polys: &[u16]) -> Decoder {
        let poly_table = util::conv_poly_table(rate, order, polys);
        let max_error = rate * u8::max_value() as u32;
        let renorm = u16::max_value() as u32 / max_error;
        let highbit = 1 << (order - 1);
        Decoder {
            rate,
            order,
            highbit,
            pair_table: ConvolutionalPairTable::new(rate, order, &poly_table),
            history_table: ConvolutionalHistoryTable::new(
                5 * order,
                15 * order,
                renorm,
                util::num_states_for_order(order) / 2,
                highbit,
            ),
            error_table: ConvolutionalErrorTable::new(
                (util::num_states_for_order(order) / 2) as usize,
            ),
            poly_table,
            distances: vec![0; 1 << rate],
        }
    }

    fn decode_warmup(&mut self, encoded: &mut BitReader) {
        // XXX todo support soft

        // we're going to prime the shift register
        for i in 0..(self.order - 1) {
            let outputs = encoded.read(self.rate as usize);

            {
                let previous_errors = &self.error_table.previous_errors;
                let errors = &mut self.error_table.errors;

                // check all reg states that are up to (not including) i + 1 bits long
                for j in 0..(1 << (i + 1)) {
                    let previous_state = j >> 1;

                    let distance = util::metric_distance(self.poly_table[j].into(), outputs.into());

                    errors[j] = distance as u16 + previous_errors[previous_state];
                }
            }
            self.error_table.swap();
        }
    }

    fn decode_inner(
        &mut self,
        encoded: &mut BitReader,
        num_encoded_bits: usize,
        decoded: &mut BitWriter,
    ) {
        // decode all bits except first (warmup) and last (tail)
        let num_decoded_bits: u32 = num_encoded_bits as u32 / self.rate;
        for _ in (self.order - 1)..(num_decoded_bits - self.order + 1) {
            let outputs = encoded.read(self.rate as usize);

            for (j, distance) in self.distances.iter_mut().enumerate() {
                *distance = util::metric_distance(j as u32, outputs.into()) as u16;
            }

            unsafe {
                {
                    self.pair_table.distances(&self.distances);
                    let pair_keys = &self.pair_table.keys;
                    let mut low_key_iter = pair_keys.iter();
                    let mut high_key_iter = pair_keys[(self.highbit as usize >> 1)..].iter();
                    let pair_distances = &mut self.pair_table.distances;

                    let previous_errors = &self.error_table.previous_errors;
                    let mut errors = &mut self.error_table.errors;

                    let history = self.history_table.get_slice();

                    let state_iter = (0..self.highbit as usize).step_by(8);
                    let prev_state_iter = (0..(self.highbit >> 1) as usize).step_by(4);
                    let high_prev_offset = (self.highbit >> 1) as usize;

                    for (state, prev_state) in state_iter.zip(prev_state_iter) {
                        for (state_offset, prev_offset) in (0..8).step_by(2).zip(0..4) {
                            let low_key = *low_key_iter.next().unwrap();
                            let high_key = *high_key_iter.next().unwrap();

                            let low_concat_distance =
                                *pair_distances.get_unchecked(low_key as usize);
                            let high_concat_distance =
                                *pair_distances.get_unchecked(high_key as usize);

                            let low_prev_error =
                                *previous_errors.get_unchecked(prev_state + prev_offset);
                            let high_prev_error = *previous_errors
                                .get_unchecked(prev_state + prev_offset + high_prev_offset);

                            let low_error: u16 =
                                (low_concat_distance & 0xffff) as u16 + low_prev_error;
                            let high_error: u16 =
                                (high_concat_distance & 0xffff) as u16 + high_prev_error;

                            let error: u16;
                            let successor: u8;
                            if low_error <= high_error {
                                error = low_error;
                                successor = 0;
                            } else {
                                error = high_error;
                                successor = 1;
                            }
                            *errors.get_unchecked_mut(state + state_offset) = error;
                            *history.get_unchecked_mut(state + state_offset) = successor;

                            let state = state + 1;

                            let low_error = (low_concat_distance >> 16) as u16 + low_prev_error;
                            let high_error = (high_concat_distance >> 16) as u16 + high_prev_error;

                            let error: u16;
                            let successor: u8;
                            if low_error <= high_error {
                                error = low_error;
                                successor = 0;
                            } else {
                                error = high_error;
                                successor = 1;
                            }
                            *errors.get_unchecked_mut(state + state_offset) = error;
                            *history.get_unchecked_mut(state + state_offset) = successor;
                        }
                    }
                }
                self.history_table
                    .process(&mut self.error_table.errors, decoded);
            }
            self.error_table.swap();
        }
    }

    fn decode_tail(
        &mut self,
        encoded: &mut BitReader,
        num_encoded_bits: usize,
        decoded: &mut BitWriter,
    ) {
        // decode last bits
        // we know that the shift register was cleared out to 0 at the end
        let num_decoded_bits: u32 = num_encoded_bits as u32 / self.rate;
        for i in (num_decoded_bits - self.order + 1)..num_decoded_bits {
            let outputs = encoded.read(self.rate as usize);
            for (j, distance) in self.distances.iter_mut().enumerate() {
                *distance = util::metric_distance(j as u32, outputs.into()) as u16;
            }

            {
                let step = 1 << (self.order - (num_decoded_bits - i));
                {
                    let previous_errors = &self.error_table.previous_errors;
                    let mut errors = &mut self.error_table.errors;

                    let history = self.history_table.get_slice();

                    let state_iter = (0..self.highbit as usize).step_by(step);
                    let prev_state_iter = (0..(self.highbit >> 1) as usize).step_by(step / 2);
                    let high_prev_offset = (self.highbit >> 1) as usize;

                    for (state, prev_state) in state_iter.zip(prev_state_iter) {
                        let low_output = self.poly_table[state];
                        let high_output = self.poly_table[state + self.highbit as usize];

                        let low_prev_error = previous_errors[prev_state];
                        let high_prev_error = previous_errors[prev_state + high_prev_offset];

                        let low_error = self.distances[low_output as usize] + low_prev_error;
                        let high_error = self.distances[high_output as usize] + high_prev_error;

                        let error: u16;
                        let successor: u8;
                        if low_error <= high_error {
                            error = low_error;
                            successor = 0;
                        } else {
                            error = high_error;
                            successor = 1;
                        }
                        errors[state] = error;
                        history[state] = successor;
                    }
                }

                self.history_table
                    .process_step(step as u32, &mut self.error_table.errors, decoded);
            }
            self.error_table.swap();
        }
    }

    pub fn decode(&mut self, encoded: &[u8], num_encoded_bits: usize, msg: &mut [u8]) -> isize {
        if num_encoded_bits as u32 % self.rate != 0 {
            return -1;
        }

        let mut bit_reader = BitReader::new(encoded);
        let mut bit_writer = BitWriter::new(msg);

        self.error_table.reset();
        self.history_table.reset();

        self.decode_warmup(&mut bit_reader);
        self.decode_inner(&mut bit_reader, num_encoded_bits, &mut bit_writer);
        self.decode_tail(&mut bit_reader, num_encoded_bits, &mut bit_writer);

        self.history_table.flush(&mut bit_writer);

        bit_writer.len() as isize
    }
}

#[derive(Debug)]
struct ConvolutionalErrorTable {
    errors: Vec<u16>,
    previous_errors: Vec<u16>,
}

impl ConvolutionalErrorTable {
    pub fn new(num_states: usize) -> ConvolutionalErrorTable {
        ConvolutionalErrorTable {
            errors: vec![0; num_states],
            previous_errors: vec![0; num_states],
        }
    }

    pub fn swap(&mut self) {
        mem::swap(&mut self.errors, &mut self.previous_errors);
    }

    pub fn reset(&mut self) {
        self.errors = vec![0; self.errors.len()];
        self.previous_errors = vec![0; self.previous_errors.len()];
    }
}

#[derive(Debug)]
struct ConvolutionalHistoryTable {
    min_traceback_length: u32,
    num_states: u32,
    highbit: u16,
    history: Vec<u8>,
    decode_buf: Vec<u8>,
    history_index: usize,
    history_len: usize,
    history_cap: usize,
    renormalize_interval: u32,
    renormalize_counter: u32,
}

impl ConvolutionalHistoryTable {
    pub fn new(
        min_traceback_length: u32,
        traceback_group_length: u32,
        renormalize_interval: u32,
        num_states: u32,
        highbit: u16,
    ) -> ConvolutionalHistoryTable {
        let cap = min_traceback_length + traceback_group_length;

        ConvolutionalHistoryTable {
            min_traceback_length,
            num_states,
            highbit,
            history: vec![0; num_states as usize * cap as usize],
            decode_buf: vec![0; cap as usize],
            history_index: 0,
            history_len: 0,
            history_cap: cap as usize,
            renormalize_interval,
            renormalize_counter: 0,
        }
    }

    pub fn get_slice(&mut self) -> &mut [u8] {
        &mut self.history[(self.history_index * self.num_states as usize)
                              ..((self.history_index + 1) * self.num_states as usize)]
    }

    pub fn least_error_path(&self, distances: &[u16], search_every: u32) -> u16 {
        let mut best_path: u16 = 0;
        let mut least_error: u16 = 0;
        for (state, distance) in distances.iter().enumerate().step_by(search_every as usize) {
            if *distance < least_error {
                least_error = *distance;
                best_path = state as u16;
            }
        }
        best_path
    }

    pub fn renormalize(&mut self, distances: &mut [u16], least_register: u16) {
        let min_distance = distances[least_register as usize];
        for distance in distances.iter_mut() {
            *distance -= min_distance;
        }
    }

    pub fn traceback(
        &mut self,
        init_best_path: u16,
        min_traceback_length: u32,
        bit_writer: &mut BitWriter,
    ) {
        let mut index = self.history_index;
        let mut best_path = init_best_path;

        // loop 1 - rewind history table but don't collect any bits
        // these bits are still converging
        for _ in 0..min_traceback_length {
            if index == 0 {
                index = self.history_cap - 1;
            } else {
                index -= 1;
            }

            let bit = self.history[index * self.num_states as usize + best_path as usize];
            let reg_bit: u16;
            if bit == 0 {
                reg_bit = 0;
            } else {
                reg_bit = self.highbit;
            }
            best_path |= reg_bit;
            best_path >>= 1;
        }

        // loop 2 - rewind history table and collect bits
        let num_decodes = self.history_len - min_traceback_length as usize;
        for decoded in self.decode_buf.iter_mut().take(num_decodes) {
            if index == 0 {
                index = self.history_cap - 1;
            } else {
                index -= 1;
            }

            let bit = self.history[index * self.num_states as usize + best_path as usize];

            let reg_bit: u16;
            if bit == 0 {
                reg_bit = 0;
                *decoded = 0;
            } else {
                reg_bit = self.highbit;
                *decoded = 1;
            }
            best_path |= reg_bit;
            best_path >>= 1;
        }

        bit_writer.write_iter(self.decode_buf[..num_decodes].iter().rev());
        self.history_len -= num_decodes;
    }

    pub fn process_step(&mut self, step: u32, distances: &mut [u16], bit_writer: &mut BitWriter) {
        self.history_index += 1;
        if self.history_index == self.history_cap {
            self.history_index = 0;
        }

        self.renormalize_counter += 1;
        self.history_len += 1;

        if self.renormalize_counter == self.renormalize_interval {
            self.renormalize_counter = 0;
            let best_path = self.least_error_path(distances, step);
            self.renormalize(distances, best_path);
            if self.history_len == self.history_cap {
                let min_traceback_length = self.min_traceback_length;
                self.traceback(best_path, min_traceback_length, bit_writer);
            }
        } else if self.history_len == self.history_cap {
            let best_path = self.least_error_path(distances, step);
            let min_traceback_length = self.min_traceback_length;
            self.traceback(best_path, min_traceback_length, bit_writer);
        }
    }

    pub fn process(&mut self, distances: &mut [u16], bit_writer: &mut BitWriter) {
        self.process_step(1, distances, bit_writer)
    }

    pub fn flush(&mut self, bit_writer: &mut BitWriter) {
        self.traceback(0, 0, bit_writer)
    }

    pub fn reset(&mut self) {
        self.history_len = 0;
        self.history_index = 0;
    }
}

#[derive(Debug)]
/// Represent convolutional distance metrics for a pair of shift register states
struct ConvolutionalPairTable {
    keys: Vec<u32>,
    outputs: Vec<u32>,
    distances: Vec<u32>,
    output_mask: u32,
    output_width: u32,
}

impl ConvolutionalPairTable {
    pub fn new(rate: u32, order: u32, poly_table: &[u16]) -> ConvolutionalPairTable {
        let num_pairs = util::num_states_for_order(order) / 2;
        let mut keys = vec![0u32; num_pairs as usize];

        let mut outputs: Vec<u32> = Vec::new();
        let mut outputs_lookup: HashMap<u32, u32> = HashMap::new();

        for (pairs, key) in poly_table.chunks(2).zip(&mut keys) {
            let output: u32 = ((pairs[1] as u32) << rate) | pairs[0] as u32;

            if !outputs_lookup.contains_key(&output) {
                outputs_lookup.insert(output, outputs.len() as u32);
                outputs.push(output);
            }
            *key = outputs_lookup[&output];
        }

        ConvolutionalPairTable {
            keys: keys,
            distances: vec![0u32; outputs.len()],
            outputs: outputs,
            output_mask: (1 << rate) - 1,
            output_width: rate,
        }
    }

    pub fn distances(&mut self, distances: &[u16]) -> &[u32] {
        for (distance, pair) in self.distances.iter_mut().zip(&self.outputs) {
            let first: u32 = pair & self.output_mask;
            let second: u32 = pair >> self.output_width;

            *distance =
                ((distances[second as usize]) as u32) << 16 | distances[first as usize] as u32;
        }
        &self.distances
    }
}
