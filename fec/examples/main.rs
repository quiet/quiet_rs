extern crate fec;

use fec::convolutional;

fn main() {
    let bytes: [u8; 10] = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
    let polys: [u16; 2] = [0o161, 0o127];
    let mut enc = convolutional::Encoder::new(2, 7, &polys);

    let enc_len = enc.encode_len(bytes.len());
    let enc_len_bytes = (enc_len) / 8 + 1;
    let mut encoded = vec![0; enc_len_bytes];

    enc.encode(&bytes, &mut encoded);

    println!("enc len {}", enc_len);

    for elem in &encoded {
        print!("{:02x?} ", elem);
    }
    println!("");

    let mut decoder = convolutional::Decoder::new(2, 7, &polys);

    let mut decoded = vec![0; enc_len_bytes / 2];

    let decoded_len = decoder.decode(&encoded, enc_len, &mut decoded);

    println!("{}", decoded_len);
    println!("{:02x?}", decoded);
}
