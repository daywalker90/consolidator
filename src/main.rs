use std::sync::{Arc, Mutex};

use anyhow::anyhow;
use cln_plugin::{
    options::{ConfigOption, DefaultIntegerConfigOption, DefaultStringConfigOption},
    Builder,
};
use consolidate::{consolidate, consolidate_below, consolidate_cancel};
use parse::check_options;
use tokio::sync::watch::{channel, Sender};

mod consolidate;
mod parse;

const OPT_CONSOLIDATE_INTERVAL: &str = "consolidator-interval";
const OPT_CONSOLIDATE_FEE_MULTI: &str = "consolidator-feemulti";
const OPT_FEE_BLOCKCOUNT: u32 = 6;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), anyhow::Error> {
    std::env::set_var("CLN_PLUGIN_LOG", "cln_plugin=info,cln_rpc=info,debug");
    log_panics::init();

    let opt_consolidate_interval: DefaultIntegerConfigOption = ConfigOption::new_i64_with_default(
        OPT_CONSOLIDATE_INTERVAL,
        3600,
        "Interval for trying to consolidate with consolidate-below, defaults to 3600s",
    );
    let opt_consolidate_fee_multi: DefaultStringConfigOption = ConfigOption::new_str_with_default(
        OPT_CONSOLIDATE_FEE_MULTI,
        "1.1",
        "Fee multiplier applied to the actual consolidation tx, \
        after the check in consolidate-below was met without the multiplier",
    );

    let confplugin = match Builder::new(tokio::io::stdin(), tokio::io::stdout())
        .rpcmethod(
            "consolidate",
            "Consolidate UTXO's with given feerate now",
            consolidate,
        )
        .rpcmethod(
            "consolidate-below",
            "Wait for feerate to drop below given rate and consolidate then",
            consolidate_below,
        )
        .rpcmethod(
            "consolidate-cancel",
            "Cancel the current consolidate-below task",
            consolidate_cancel,
        )
        .option(opt_consolidate_interval)
        .option(opt_consolidate_fee_multi)
        .dynamic()
        .configure()
        .await?
    {
        Some(plugin) => {
            if let Err(e) = check_options(&plugin) {
                plugin.disable(&e.to_string()).await?;
                return Ok(());
            } else {
                plugin
            }
        }
        None => return Err(anyhow!("Error configuring the plugin!")),
    };
    let state = PluginState {
        consolidate_lock: Arc::new(Mutex::new(false)),
        consolidate_cancel: Arc::new(channel(false).0),
    };
    if let Ok(plugin) = confplugin.start(state).await {
        plugin.join().await
    } else {
        Err(anyhow!("Error starting the plugin!"))
    }
}

#[derive(Debug, Clone)]
pub struct PluginState {
    consolidate_lock: Arc<Mutex<bool>>,
    consolidate_cancel: Arc<Sender<bool>>,
}
