pub fn num_states_for_order(order: u32) -> u32 {
    1 << order
}

pub fn metric_distance(x: u32, y: u32) -> u32 {
    (x ^ y).count_ones()
}

pub fn conv_poly_table(rate: u32, order: u32, polys: &[u16]) -> Vec<u16> {
    let num_states = 1 << order;
    let mut table = vec![0; num_states];
    for i in 0..num_states {
        let mut concat: u16 = 0;
        let mut mask: u16 = 1;
        for j in 0..rate {
            let poly: u16 = polys[j as usize];
            let state_and_poly = i as u16 & poly;
            if state_and_poly.count_ones() % 2 == 1 {
                concat |= mask;
            }
            mask <<= 1;
        }
        table[i] = concat;
    }
    table
}
