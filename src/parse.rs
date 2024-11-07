use anyhow::{anyhow, Error};
use cln_rpc::model::responses::FeeratesPerkb;

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
                feerates.opening.ok_or(anyhow!(
                    "No feerate provided by user and no feerate provided by CLN"
                ))?
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
                feerates.opening.ok_or(anyhow!(
                    "No feerate provided by user and no feerate provided by CLN"
                ))?
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
