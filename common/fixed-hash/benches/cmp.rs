// Copyright 2020 Parity Technologies
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Benchmarks for fixed-hash cmp implementation.

use criterion::{black_box, criterion_group, criterion_main, Criterion, ParameterizedBenchmark};

use fixed_hash::construct_fixed_hash;

construct_fixed_hash! { pub struct H256(32); }

criterion_group!(cmp, eq_equal, eq_nonequal, compare,);
criterion_main!(cmp);

fn eq_equal(c: &mut Criterion) {
    c.bench(
        "eq_equal",
        ParameterizedBenchmark::new(
            "",
            |b, x| b.iter(|| black_box(x.eq(black_box(x)))),
            vec![
                H256::zero(),
                H256::repeat_byte(0xAA),
                H256::from([
                    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xEF, 0x2D, 0x6D, 0x19,
                    0x40, 0x84, 0xC2, 0xDE, 0x36, 0xE0, 0xDA, 0xBF, 0xCE, 0x45, 0xD0, 0x46, 0xB3, 0x7D, 0x11, 0x06,
                ]),
                H256([u8::max_value(); 32]),
            ],
        ),
    );
}

fn eq_nonequal(c: &mut Criterion) {
    c.bench(
        "eq_nonequal",
        ParameterizedBenchmark::new(
            "",
            |b, (x, y)| b.iter(|| black_box(x.eq(black_box(y)))),
            vec![
                (
                    H256::zero(),
                    H256::from([
                        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1,
                    ]),
                ),
                (H256::repeat_byte(0xAA), H256::repeat_byte(0xA1)),
                (
                    H256::from([
                        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xEF, 0x2D, 0x6D, 0x19,
                        0x40, 0x84, 0xC2, 0xDE, 0x36, 0xE0, 0xDA, 0xBF, 0xCE, 0x45, 0xD0, 0x46, 0xB3, 0x7D, 0x11, 0x06,
                    ]),
                    H256::from([
                        0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xEF, 0x2D, 0x6D, 0x19,
                        0x40, 0x84, 0xC2, 0xDE, 0x36, 0xE0, 0xDA, 0xBF, 0xCE, 0x45, 0xD0, 0x46, 0xB3, 0x7D, 0x11, 0x06,
                    ]),
                ),
            ],
        ),
    );
}

fn compare(c: &mut Criterion) {
    c.bench(
        "compare",
        ParameterizedBenchmark::new(
            "",
            |b, (x, y)| b.iter(|| black_box(x.cmp(black_box(y)))),
            vec![
                (
                    H256::zero(),
                    H256::from([
                        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1,
                    ]),
                ),
                (H256::zero(), H256::zero()),
                (H256::repeat_byte(0xAA), H256::repeat_byte(0xAA)),
                (H256::repeat_byte(0xAA), H256::repeat_byte(0xA1)),
                (
                    H256::from([
                        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xEF, 0x2D, 0x6D, 0x19,
                        0x40, 0x84, 0xC2, 0xDF, 0x36, 0xE0, 0xDA, 0xBF, 0xCE, 0x45, 0xD0, 0x46, 0xB3, 0x7D, 0x11, 0x06,
                    ]),
                    H256::from([
                        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xEF, 0x2D, 0x6D, 0x19,
                        0x40, 0x84, 0xC2, 0xDE, 0x36, 0xE0, 0xDA, 0xBF, 0xCE, 0x45, 0xD0, 0x46, 0xB3, 0x7D, 0x11, 0x06,
                    ]),
                ),
            ],
        ),
    );
}
