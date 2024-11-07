use std::sync::{Arc, Mutex};

use anyhow::anyhow;
use cln_plugin::{
    options::{ConfigOption, DefaultIntegerConfigOption},
    Builder,
};
use consolidate::{consolidate, consolidate_below, consolidate_cancel};
use tokio::sync::watch::{channel, Sender};

mod consolidate;
mod parse;

const OPT_CONSOLIDATE_INTERVAL: &str = "consolidator-interval";

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), anyhow::Error> {
    std::env::set_var("CLN_PLUGIN_LOG", "cln_plugin=info,cln_rpc=info,debug");
    log_panics::init();

    let opt_consolidate_interval: DefaultIntegerConfigOption = ConfigOption::new_i64_with_default(
        OPT_CONSOLIDATE_INTERVAL,
        3600,
        "Interval for trying to consolidate with consolidate-below, defaults to 3600s",
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
        .dynamic()
        .configure()
        .await?
    {
        Some(plugin) => plugin,
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
