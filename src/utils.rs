use std::ops::Div;
use std::str::FromStr;

use bigdecimal::BigDecimal;
use num_bigint::BigUint;
use pad::PadStr;
use std::convert::TryInto;
use substreams::{proto, store, Hex};

use crate::pb::uniswap;

pub fn convert_token_to_decimal(amount: BigUint, decimals: &u64) -> BigDecimal {
    let big_float_amount = BigDecimal::from_str(amount.to_string().as_str())
        .unwrap()
        .with_prec(100);

    return divide_by_decimals(big_float_amount, decimals);
}

pub fn get_token_price(bf0: BigDecimal, bf1: BigDecimal) -> BigDecimal {
    return bf0.div(bf1).with_prec(100);
}

pub fn get_last_token(tokens: &store::StoreGet, address: &Vec<u8>) -> uniswap::Token {
    let encoded = tokens
        .get_last(&format!("token:{}", Hex(&address)))
        .unwrap();

    proto::decode(&encoded).unwrap()
}

fn divide_by_decimals(big_float_amount: BigDecimal, decimals: &u64) -> BigDecimal {
    let bd = BigDecimal::from_str(
        "1".pad_to_width_with_char((*decimals + 1) as usize, '0')
            .as_str(),
    )
    .unwrap()
    .with_prec(100);

    return big_float_amount.div(bd).with_prec(100);
}

pub fn read_uint32(input: &[u8]) -> Result<u32, String> {
    if input.len() != 32 {
        return Err(format!("uint32 invalid length: {}", input.len()));
    }
    let as_array: [u8; 4] = input[28..32].try_into().unwrap();
    Ok(u32::from_be_bytes(as_array))
}

pub fn read_string(input: &[u8]) -> Result<String, String> {
    if input.len() < 96 {
        return Err(format!("string invalid length: {}", input.len()));
    }

    let next = read_uint32(&input[0..32])?;
    if next != 32 {
        return Err(format!("invalid string uint32 value: {}", next));
    };

    let size = read_uint32(&input[32..64])?;
    let end: usize = (size as usize) + 64;

    if end > input.len() {
        return Err(format!(
            "invalid input: end {:?}, length: {:?}, next: {:?}, size: {:?}, whole: {:?}",
            end,
            input.len(),
            next,
            size,
            Hex(&input[32..64])
        ));
    }

    Ok(String::from_utf8_lossy(&input[64..end]).to_string())
}
