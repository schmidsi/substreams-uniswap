mod abi;
mod pb;
mod rpc;
mod utils;
use std::str::FromStr;

use hex_literal::hex;
use num_bigint::BigUint;
use pb::uniswap;
use substreams::{errors::Error, log, proto, store, Hex};
use substreams_ethereum::pb::eth::v1 as eth;

const FACTORY: [u8; 20] = hex!("5c69bee701ef814a2b6a3edd4b1652cb9cc5aa6f");

substreams_ethereum::init!();

#[substreams::handlers::map]
fn map_pairs(block: eth::Block) -> Result<uniswap::Pairs, Error> {
    let mut pairs = vec![];

    for tx in block.transaction_traces {
        pairs.extend(tx.receipt.unwrap().logs.iter().filter_map(|log| {
            if log.address != FACTORY || !abi::factory::events::PairCreated::match_log(&log) {
                return None;
            }

            let event = abi::factory::events::PairCreated::must_decode(&log);

            log::info!("Pair created: 0x{}", Hex(&event.pair));

            Some(uniswap::Pair {
                address: event.pair,
                token0: event.token0,
                token1: event.token1,
                ordinal: log.ordinal,
            })
        }));
    }

    Ok(uniswap::Pairs { pairs })
}

#[substreams::handlers::store]
fn store_pairs(pairs: uniswap::Pairs, store: store::StoreSet) {
    for pair in pairs.pairs {
        let key = format!("pair:{}", Hex(&pair.address));
        store.set(pair.ordinal, key, &proto::encode(&pair).unwrap());
    }
}

// TODO: This should reuse the generic token stream (ethtokens). During development, that's currently
// not practical because of various reasons (performance, weird proxy based tokens, etc.).
#[substreams::handlers::store]
fn store_tokens(pairs: uniswap::Pairs, store: store::StoreSetIfNotExists) {
    for pair in pairs.pairs {
        for address in vec![pair.token0, pair.token1] {
            let token = rpc::rpc_fetch_token(&address).unwrap();
            let key = format!("token:{}", Hex(&address));

            log::info!("Fetched token details: 0x{}", Hex(&address));

            store.set_if_not_exists(1, key, &proto::encode(&token).unwrap());
        }
    }
}

#[substreams::handlers::map]
fn map_reserves(
    block: eth::Block,
    pairs: store::StoreGet,
    tokens: store::StoreGet,
) -> Result<uniswap::Reserves, Error> {
    let mut reserves = vec![];

    for tx in block.transaction_traces {
        reserves.extend(tx.receipt.unwrap().logs.iter().filter_map(|log| {
            match pairs.get_last(&format!("pair:{}", Hex(&log.address))) {
                None => None,
                Some(encoded) => {
                    let pair: uniswap::Pair = proto::decode(&encoded).unwrap();

                    if !abi::pair::events::Sync::match_log(log) {
                        return None;
                    }

                    let event = abi::pair::events::Sync::must_decode(log);

                    let token0 = utils::get_last_token(&tokens, &pair.token0);
                    let token1 = utils::get_last_token(&tokens, &pair.token1);

                    let reserve0 = utils::convert_token_to_decimal(
                        BigUint::from_str(&event.reserve0.to_string()).unwrap(),
                        &token0.decimals,
                    );
                    let reserve1 = utils::convert_token_to_decimal(
                        BigUint::from_str(&event.reserve1.to_string()).unwrap(),
                        &token1.decimals,
                    );

                    let token0_price = utils::get_token_price(reserve0.clone(), reserve1.clone());
                    let token1_price = utils::get_token_price(reserve1.clone(), reserve0.clone());

                    Some(uniswap::Reserve {
                        ordinal: log.ordinal,
                        pair: log.address.clone(),
                        reserve0: reserve0.to_string(),
                        reserve1: reserve1.to_string(),
                        token0_price: token0_price.to_string(),
                        token1_price: token1_price.to_string(),
                    })
                }
            }
        }));
    }

    Ok(uniswap::Reserves { reserves })
}
