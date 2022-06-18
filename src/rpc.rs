use substreams::Hex;
use substreams_ethereum::{pb::eth, rpc};

use crate::{
    pb::uniswap::Token,
    utils::{read_string, read_uint32},
};

fn create_rpc_calls(addr: &Vec<u8>) -> eth::rpc::RpcCalls {
    let decimals = hex::decode("313ce567").unwrap();
    let name = hex::decode("06fdde03").unwrap();
    let symbol = hex::decode("95d89b41").unwrap();

    return eth::rpc::RpcCalls {
        calls: vec![
            eth::rpc::RpcCall {
                to_addr: Vec::from(addr.clone()),
                method_signature: decimals,
            },
            eth::rpc::RpcCall {
                to_addr: Vec::from(addr.clone()),
                method_signature: name,
            },
            eth::rpc::RpcCall {
                to_addr: Vec::from(addr.clone()),
                method_signature: symbol,
            },
        ],
    };
}

pub fn rpc_fetch_token(address: &Vec<u8>) -> Result<Token, String> {
    let rpc_calls = create_rpc_calls(address);

    let unmarshalled: eth::rpc::RpcResponses = rpc::eth_call(&rpc_calls);
    let responses = unmarshalled.responses;

    if responses[0].failed || responses[1].failed || responses[2].failed {
        return Err(format!("0x{} is not an ERC20 token", Hex(address)));
    };

    let decoded_decimals = read_uint32(responses[0].raw.as_ref());
    if decoded_decimals.is_err() {
        return Err(format!(
            "0x{} decimal `eth_call` failed: {}",
            Hex(address),
            decoded_decimals.err().unwrap()
        ));
    }

    let decoded_name = read_string(responses[1].raw.as_ref());
    if decoded_name.is_err() {
        return Err(format!(
            "0x{} name `eth_call` failed: {}",
            Hex(address),
            decoded_name.err().unwrap()
        ));
    }

    let decoded_symbol = read_string(responses[2].raw.as_ref());
    if decoded_symbol.is_err() {
        return Err(format!(
            "0x{} symbol `eth_call` failed: {}",
            Hex(address),
            decoded_symbol.err().unwrap()
        ));
    }

    return Ok(Token {
        address: address.clone(),
        name: decoded_name.unwrap(),
        symbol: decoded_symbol.unwrap(),
        decimals: decoded_decimals.unwrap() as u64,
    });
}
