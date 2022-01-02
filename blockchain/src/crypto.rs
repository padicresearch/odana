#[cfg(test)]
mod tests {
    use num_bigint::BigUint;

    #[test]
    fn difficulty() {
        let mut target_number_bytes = [0; 32];
        println!(
            "Target {}, :{:?}",
            BigUint::from_bytes_be(&target_number_bytes[..]),
            target_number_bytes
        );
        println!("Target Hex {}", hex::encode(target_number_bytes));

        target_number_bytes[2] = 125;
        target_number_bytes[3] = u8::MAX;

        println!(
            "Target {}, :{:?}",
            BigUint::from_bytes_be(&target_number_bytes[..]),
            target_number_bytes
        );
        println!("Target Hex {}", hex::encode(target_number_bytes));
        //let mut target_number_bytes = [u8::MAX;32];
        //println!("Target {}, :{:?}", BigUint::from_bytes_be(&target_number_bytes[..]),Big target_number_bytes);
    }
}
