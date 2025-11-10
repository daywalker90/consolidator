use std::{
    path::Path,
    sync::{Arc, Mutex},
};

use anyhow::anyhow;
use cln_plugin::{
    options::{
        ConfigOption,
        DefaultBooleanConfigOption,
        DefaultIntegerConfigOption,
        DefaultStringConfigOption,
    },
    Builder,
};
use cln_rpc::ClnRpc;
use consolidate::{consolidate, consolidate_below, consolidate_cancel, load_consolidate};
use parse::check_options;
use tokio::sync::watch::{channel, Sender};

mod consolidate;
mod parse;

const OPT_CONSOLIDATE_INTERVAL: DefaultIntegerConfigOption = ConfigOption::new_i64_with_default(
    "consolidator-interval",
    3600,
    "Interval for trying to consolidate with consolidate-below, defaults to 3600s",
);
const OPT_CONSOLIDATE_FEE_MULTI: DefaultStringConfigOption = ConfigOption::new_str_with_default(
    "consolidator-feemulti",
    "1.1",
    "Fee multiplier applied to the actual consolidation tx, \
    after the check in consolidate-below was met without the multiplier",
);
const OPT_CONSOLIDATE_PERSIST: DefaultBooleanConfigOption = ConfigOption::new_bool_with_default(
    "consolidator-persist",
    false,
    "If a `consolidate-below` should be persistent betweend plugin/node restarts.",
);
const OPT_FEE_BLOCKCOUNT: u32 = 6;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), anyhow::Error> {
    std::env::set_var("CLN_PLUGIN_LOG", "cln_plugin=info,cln_rpc=info,debug");
    log_panics::init();

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
        .option(OPT_CONSOLIDATE_INTERVAL)
        .option(OPT_CONSOLIDATE_FEE_MULTI)
        .option(OPT_CONSOLIDATE_PERSIST)
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
        if plugin.option(&OPT_CONSOLIDATE_PERSIST).unwrap() {
            let mut rpc = ClnRpc::new(
                Path::new(&plugin.configuration().lightning_dir)
                    .join(&plugin.configuration().rpc_file),
            )
            .await?;
            match load_consolidate(&mut rpc).await {
                Ok(args) => match consolidate_below(plugin.clone(), args.clone()).await {
                    Ok(_co) => log::info!(
                        "Successfully started saved consolidate-below command with: {}.",
                        args
                    ),
                    Err(ce) => log::info!("Error starting saved consolidate-below command: {}", ce),
                },
                Err(e) => log::info!("Loading persisted consolidate command: {}", e),
            };
        }

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
