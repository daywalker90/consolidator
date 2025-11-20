use std::{
    path::Path,
    str::FromStr,
    time::{Duration, Instant},
};

use anyhow::anyhow;
use bitcoin::hashes::sha256::Hash as Sha256;
use cln_plugin::Plugin;
use cln_rpc::{
    model::{
        requests::{
            DatastoreMode,
            DatastoreRequest,
            DeldatastoreRequest,
            FeeratesRequest,
            FeeratesStyle,
            ListdatastoreRequest,
            ListfundsRequest,
            NewaddrAddresstype,
            NewaddrRequest,
            WithdrawRequest,
        },
        responses::ListfundsOutputsStatus,
    },
    primitives::{AmountOrAll, Feerate, Outpoint},
    ClnRpc,
};
use serde_json::json;
use tokio::{task, time};

use crate::{
    parse::{get_blockcount_feerate, parse_consolidate_args},
    PluginState,
    OPT_CONSOLIDATE_FEE_MULTI,
    OPT_CONSOLIDATE_INTERVAL,
    OPT_CONSOLIDATE_PERSIST,
    OPT_FEE_BLOCKCOUNT,
};

pub async fn consolidate(
    plugin: Plugin<PluginState>,
    args: serde_json::Value,
) -> Result<serde_json::Value, anyhow::Error> {
    let mut rpc = ClnRpc::new(
        Path::new(&plugin.configuration().lightning_dir).join(&plugin.configuration().rpc_file),
    )
    .await?;
    let feerates = rpc
        .call_typed(&FeeratesRequest {
            style: FeeratesStyle::PERKB,
        })
        .await?
        .perkb
        .ok_or(anyhow!("Feerates did not return perkb object"))?;
    let (feerate, min_utxos_count) = parse_consolidate_args(&args, &feerates)?;

    let mut utxos = rpc
        .call_typed(&ListfundsRequest { spent: Some(false) })
        .await?
        .outputs;

    let raw_configs: serde_json::Value = rpc.call_raw("listconfigs", &json!({})).await?;
    let emergency_msat = raw_configs
        .get("configs")
        .ok_or(anyhow!("malformed configs response"))?
        .get("min-emergency-msat")
        .ok_or(anyhow!("min-emergency-msat field empty"))?
        .get("value_msat")
        .ok_or(anyhow!("min-emergency-msat value not found"))?
        .as_u64()
        .ok_or(anyhow!("min-emergency-msat not a number"))?;
    let mut cons_utxos: Vec<Outpoint> = Vec::new();

    utxos.sort_by_key(|u| u.amount_msat.msat());
    let mut emerg_utxo_found = false;
    for utxo in &utxos {
        if utxo.reserved {
            continue;
        }
        if utxo.status != ListfundsOutputsStatus::CONFIRMED {
            continue;
        }
        if !emerg_utxo_found && utxo.amount_msat.msat() >= emergency_msat {
            emerg_utxo_found = true;
            continue;
        }
        if 70 * u64::from(feerate) > utxo.amount_msat.msat() {
            continue;
        }
        cons_utxos.push(Outpoint {
            txid: Sha256::from_str(&utxo.txid)?,
            outnum: utxo.output,
        });
    }

    if cons_utxos.len() < min_utxos_count {
        return Err(anyhow!(
            "Not enough UTXO's to consolidate: Current:{} Wanted:>={}",
            cons_utxos.len(),
            min_utxos_count
        ));
    }

    let destination = rpc
        .call_typed(&NewaddrRequest {
            addresstype: Some(NewaddrAddresstype::P2TR),
        })
        .await?
        .p2tr
        .ok_or(anyhow!("Could not get p2tr address"))?;
    let withdraw = rpc
        .call_typed(&WithdrawRequest {
            feerate: Some(Feerate::PerKb(feerate)),
            minconf: None,
            utxos: Some(cons_utxos.clone()),
            destination,
            satoshi: AmountOrAll::All,
        })
        .await?;
    // log::debug!(
    //     "utxos:{:?}, feerate:{}, cons_utxos:{:?}",
    //     utxos,
    //     feerate,
    //     cons_utxos
    // );
    // log::debug!("tx:{}", withdraw.tx);
    Ok(json!({"num_utxos_consolidating":cons_utxos.len(),"tx":withdraw.tx,"txid":withdraw.txid}))
}

pub async fn consolidate_below(
    plugin: Plugin<PluginState>,
    args: serde_json::Value,
) -> Result<serde_json::Value, anyhow::Error> {
    let mut rpc = ClnRpc::new(
        Path::new(&plugin.configuration().lightning_dir).join(&plugin.configuration().rpc_file),
    )
    .await?;

    {
        let mut is_running = plugin.state().consolidate_lock.lock().unwrap();
        if *is_running {
            return Err(anyhow!("Already have a consolidate-below running!"));
        }
        *is_running = true;
    }

    if plugin.option(&OPT_CONSOLIDATE_PERSIST).unwrap() {
        save_consolidate(&mut rpc, args.clone()).await?;
    }

    task::spawn(async move {
        let interval = u64::try_from(plugin.option(&OPT_CONSOLIDATE_INTERVAL).unwrap()).unwrap();
        let cancel_rx = plugin.state().consolidate_cancel.subscribe();
        plugin.state().consolidate_cancel.send(false).unwrap();
        let mut now = Instant::now();
        let mut first_run = true;
        loop {
            if *cancel_rx.borrow() {
                log::info!("consolidate_below CANCELED");
                *plugin.state().consolidate_lock.lock().unwrap() = false;
                _ = del_consolidate(&mut rpc).await;
                break;
            }
            if !first_run && now.elapsed().as_secs() < interval {
                time::sleep(Duration::from_millis(200)).await;
                continue;
            }
            now = Instant::now();
            first_run = false;

            let Ok(feerates_resp) = rpc
                .call_typed(&FeeratesRequest {
                    style: FeeratesStyle::PERKB,
                })
                .await
            else {
                log::info!("consolidate_below: Could not get feerates");
                continue;
            };
            let Some(feerates) = feerates_resp.perkb else {
                log::info!("consolidate_below: Feerates did not return perkb object");
                continue;
            };
            let (feerate, min_utxos_count) = match parse_consolidate_args(&args, &feerates) {
                Ok((f, c)) => (f, c),
                Err(e) => {
                    log::info!("consolidate_below: {e}");
                    continue;
                }
            };
            let blkcnt6_feerate = match get_blockcount_feerate(&feerates, OPT_FEE_BLOCKCOUNT) {
                Ok(fr) => fr,
                Err(e) => {
                    log::info!("consolidate_below: {e}");
                    continue;
                }
            };
            if blkcnt6_feerate < feerate {
                let fee_multi = plugin
                    .option(&OPT_CONSOLIDATE_FEE_MULTI)
                    .unwrap()
                    .parse::<f64>()
                    .unwrap();
                match consolidate(
                    plugin.clone(),
                    json!({"feerate":(f64::from(blkcnt6_feerate)*fee_multi).round() as u64,
                           "min_utxos":min_utxos_count}),
                )
                .await
                {
                    Ok(o) => {
                        log::info!("consolidate_below: SUCCESS: {o}");
                        *plugin.state().consolidate_lock.lock().unwrap() = false;
                        _ = del_consolidate(&mut rpc).await;
                        break;
                    }
                    Err(e) => {
                        log::info!("consolidate_below: {e}");
                    }
                }
            } else {
                log::info!(
                    "Feerate not low enough yet: \
                    Current:{blkcnt6_feerate}perkb Wanted:<{feerate}perkb"
                );
            }
        }
    });

    Ok(json!({"result":"OK"}))
}

pub async fn consolidate_cancel(
    plugin: Plugin<PluginState>,
    _args: serde_json::Value,
) -> Result<serde_json::Value, anyhow::Error> {
    plugin.state().consolidate_cancel.send(true)?;
    Ok(json!({"result":"Canceled"}))
}

async fn save_consolidate(rpc: &mut ClnRpc, args: serde_json::Value) -> Result<(), anyhow::Error> {
    rpc.call_typed(&DatastoreRequest {
        generation: None,
        hex: None,
        mode: Some(DatastoreMode::CREATE_OR_REPLACE),
        string: Some(args.to_string()),
        key: vec!["consolidator".to_owned(), "consolidate-below".to_owned()],
    })
    .await?;
    Ok(())
}

async fn del_consolidate(rpc: &mut ClnRpc) -> Result<(), anyhow::Error> {
    rpc.call_typed(&DeldatastoreRequest {
        generation: None,
        key: vec!["consolidator".to_owned(), "consolidate-below".to_owned()],
    })
    .await?;
    Ok(())
}

pub async fn load_consolidate(rpc: &mut ClnRpc) -> Result<serde_json::Value, anyhow::Error> {
    let result = rpc
        .call_typed(&ListdatastoreRequest {
            key: Some(vec![
                "consolidator".to_owned(),
                "consolidate-below".to_owned(),
            ]),
        })
        .await?;
    let str_stored = result
        .datastore
        .first()
        .ok_or_else(|| anyhow!("No consolidate job found"))?
        .string
        .clone()
        .ok_or_else(|| anyhow!("No consolidate arguments found in db"))?;
    serde_json::from_str(&str_stored).map_err(|e| anyhow!(e))
}
