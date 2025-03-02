use anyhow::{anyhow, Context, Error};
use cln_plugin::ConfiguredPlugin;
use cln_rpc::model::responses::FeeratesPerkb;

use crate::{PluginState, OPT_CONSOLIDATE_FEE_MULTI, OPT_CONSOLIDATE_INTERVAL, OPT_FEE_BLOCKCOUNT};

pub fn parse_consolidate_args(
    args: &serde_json::Value,
    feerates: &FeeratesPerkb,
) -> Result<(u32, usize), Error> {
    let (feerate, min_utxos_count) = match args {
        serde_json::Value::Array(fra) => {
            if fra.len() > 2 {
                return Err(anyhow!("Too many arguments!"));
            }
            let feerate = if let Some(afr) = fra.first() {
                u32::try_from(afr.as_u64().ok_or(anyhow!("Not a valid feerate number"))?)?
            } else {
                get_blockcount_feerate(feerates, OPT_FEE_BLOCKCOUNT)
                    .context("No feerate provided by user and no feerate provided by CLN")?
            };
            let min_utxos_count = if let Some(muc) = fra.get(1) {
                usize::try_from(
                    muc.as_u64()
                        .ok_or(anyhow!("Not a valid number for minimum utxo count"))?,
                )?
            } else {
                10
            };
            (feerate, min_utxos_count)
        }
        serde_json::Value::Object(fro) => {
            let feerate = if let Some(ofr) = fro.get("feerate") {
                u32::try_from(ofr.as_u64().ok_or(anyhow!("Not a valid feerate number"))?)?
            } else {
                get_blockcount_feerate(feerates, OPT_FEE_BLOCKCOUNT)
                    .context("No feerate provided by user and no feerate provided by CLN")?
            };
            let min_utxos_count = if let Some(muc) = fro.get("min_utxos") {
                usize::try_from(
                    muc.as_u64()
                        .ok_or(anyhow!("Not a valid number for minimum utxo count"))?,
                )?
            } else {
                10
            };
            (feerate, min_utxos_count)
        }
        _ => return Err(anyhow!("Unsupported argument object")),
    };

    if feerate < feerates.min_acceptable {
        return Err(anyhow!(
            "Feerate {}perkb is below min_acceptable of {}perkb",
            feerate,
            feerates.min_acceptable
        ));
    }
    if feerate > feerates.max_acceptable {
        return Err(anyhow!(
            "Feerate {}perkb is above max_acceptable of {}perkb",
            feerate,
            feerates.max_acceptable
        ));
    }

    Ok((feerate, min_utxos_count))
}

pub fn check_options(
    plugin: &ConfiguredPlugin<PluginState, tokio::io::Stdin, tokio::io::Stdout>,
) -> Result<(), Error> {
    let fee_multi = plugin
        .option(&OPT_CONSOLIDATE_FEE_MULTI)
        .unwrap()
        .parse::<f64>()
        .context("Could not parse fee_multi as a decimal")?;
    if !(0.3..=3.0).contains(&fee_multi) {
        return Err(anyhow!(
            "{} outside of allowed range [0.3,3.0]",
            OPT_CONSOLIDATE_FEE_MULTI.name()
        ));
    }
    let interval = plugin.option(&OPT_CONSOLIDATE_INTERVAL).unwrap();
    if interval < 1 {
        return Err(anyhow!(
            "{} outside of valid range [1,{}]",
            OPT_CONSOLIDATE_INTERVAL.name(),
            u64::MAX
        ));
    }
    Ok(())
}

pub fn get_blockcount_feerate(feerates: &FeeratesPerkb, blockcount: u32) -> Result<u32, Error> {
    let estimates = if let Some(est) = &feerates.estimates {
        est
    } else {
        return Err(anyhow!(
            "Feerates perkb object did not contain the estimates object"
        ));
    };
    for estimate in estimates {
        if estimate.blockcount == blockcount {
            return Ok(estimate.feerate);
        }
    }

    Err(anyhow!(
        "Feerates perkb object did not contain \
        blockcount:{} feerate",
        blockcount
    ))
}
