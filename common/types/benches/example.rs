use codec::{ConsensusCodec, Encoder};
use criterion::{criterion_group, criterion_main, Criterion};
use primitive_types::{H160, H256, U128, U256};
use types::block::BlockHeader;

fn consensus_codec() -> usize {
    let block_header = BlockHeader::new(
        H256::from([1; 32]),
        H256::from([2; 32]),
        H256::from([6; 32]),
        U256::from(400),
        H160::from([7; 20]),
        30,
        30,
        30,
        10000000,
        U128::from(5),
    );

    let pheader = block_header.consensus_encode();
    pheader.len()
}

fn bincodec() -> usize {
    let block_header = BlockHeader::new(
        H256::from([1; 32]),
        H256::from([2; 32]),
        H256::from([6; 32]),
        U256::from(400),
        H160::from([7; 20]),
        30,
        30,
        30,
        10000000,
        U128::from(5),
    );

    let pheader = block_header.encode().unwrap();
    pheader.len()
}

//#[bench]
fn bench_codec(b: &mut Criterion) {
    b.bench_function("bincodec", |b| b.iter(|| bincodec()));
}

//#[bench]
fn bench_consensus_codec(b: &mut Criterion) {
    b.bench_function("consensus_codec", |b| b.iter(|| consensus_codec()));
}

criterion_group!(benches, bench_codec, bench_consensus_codec);
criterion_main!(benches);
