use std::sync::Arc;

use alloy_primitives::{Address, Bytes};
use alloy_sol_types::SolCall;
use brontes_core::missing_token_info::load_missing_token_info;
use brontes_types::{
    db::traits::{LibmdbxReader, LibmdbxWriter},
    normalized_actions::NormalizedTransfer,
    traits::TracingProvider,
    ToScaledRational,
};
use malachite::{num::basic::traits::Zero, Rational};

alloy_sol_macro::sol!(
    function transfer(address, uint) returns(bool);
    function transferFrom(address, address, uint) returns(bool);
);

pub async fn try_decode_transfer<T: TracingProvider, DB: LibmdbxReader + LibmdbxWriter>(
    idx: u64,
    calldata: Bytes,
    from: Address,
    token: Address,
    db: &DB,
    provider: &Arc<T>,
    block: u64,
) -> eyre::Result<NormalizedTransfer> {
    let Some((from_addr, to_addr, amount)) = transferCall::abi_decode(&calldata, false)
        .map(|t| Some((from, t._0, t._1)))
        .unwrap_or_else(|_| {
            transferFromCall::abi_decode(&calldata, false)
                .ok()
                .map(|t| (t._0, t._1, t._2))
        })
    else {
        return Err(eyre::eyre!("failed to decode transfer for token: {:?}", token))
    };

    if db.try_fetch_token_info(token).is_err() {
        load_missing_token_info(provider, db, block, token).await
    }

    let token_info = db.try_fetch_token_info(token)?;

    Ok(NormalizedTransfer {
        amount:      amount.to_scaled_rational(token_info.decimals),
        token:       token_info,
        to:          to_addr,
        from:        from_addr,
        trace_index: idx,
        fee:         Rational::ZERO,
    })
}
