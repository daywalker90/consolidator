[![latest release on CLN v24.08.2](https://github.com/daywalker90/consolidator/actions/workflows/latest_v24.08.yml/badge.svg?branch=main)](https://github.com/daywalker90/consolidator/actions/workflows/latest_v24.08.yml) [![latest release on CLN v24.05](https://github.com/daywalker90/consolidator/actions/workflows/latest_v24.05.yml/badge.svg?branch=main)](https://github.com/daywalker90/consolidator/actions/workflows/latest_v24.05.yml) [![latest release on CLN v24.02.1](https://github.com/daywalker90/consolidator/actions/workflows/latest_v24.02.yml/badge.svg?branch=main)](https://github.com/daywalker90/consolidator/actions/workflows/latest_v24.02.yml) 

[![main on CLN v24.08.2](https://github.com/daywalker90/consolidator/actions/workflows/main_v24.08.yml/badge.svg?branch=main)](https://github.com/daywalker90/consolidator/actions/workflows/main_v24.08.yml) [![main on CLN v24.05](https://github.com/daywalker90/consolidator/actions/workflows/main_v24.05.yml/badge.svg?branch=main)](https://github.com/daywalker90/consolidator/actions/workflows/main_v24.05.yml) [![main on CLN v24.02.1](https://github.com/daywalker90/consolidator/actions/workflows/main_v24.02.yml/badge.svg?branch=main)](https://github.com/daywalker90/consolidator/actions/workflows/main_v24.02.yml) 

# consolidator
A core lightning plugin to automatically consolidate your UTXO's.

* [Installation](#installation)
* [Building](#building)
* [Command documentation](#command-documentation)
* [Important notes](#important-notes)
* [Options](#options)

# Installation
For general plugin installation instructions see the plugins repo [README.md](https://github.com/lightningd/plugins/blob/master/README.md#Installation)

Release binaries for
* x86_64-linux
* armv7-linux (Raspberry Pi 32bit)
* aarch64-linux (Raspberry Pi 64bit)

can be found on the [release](https://github.com/daywalker90/consolidator/releases) page. If you are unsure about your architecture you can run ``uname -m``.

They require ``glibc>=2.31``, which you can check with ``ldd --version``.


# Building
You can build the plugin yourself instead of using the release binaries.
First clone the repo:

``git clone https://github.com/daywalker90/consolidator.git``

Install a recent rust version ([rustup](https://rustup.rs/) is recommended) and ``cd`` into the ``consolidator`` folder, then:

``cargo build --release``

After that the binary will be here: ``target/release/consolidator``

Note: Release binaries are built using ``cross`` and the ``optimized`` profile.

# Command documentation

* ``consolidate`` *feerate* [*min_utxos*] 

Consolidate UTXO's NOW with the given *feerate*. Optionally specify the minimum amount of UTXO's to consolidate with *min_utxos* (Default: 10)
* ``consolidate-below`` *feerate* [*min_utxos*] 

Spawn a background task to check if CLN's *opening* feerate dropped below the given *feerate* and execute a consolidation once with CLN's *opening* feerate. Optionally specify the minimum amount of UTXO's to consolidate with *min_utxos* (Default: 10). Feerate is checked every ``consolidator-interval`` seconds (Defaults to 3600, aka 1 hour)
* ``consolidate-cancel`` 

Cancel the running background task started by ``consolidate-below``

# Important notes

* Consolidator only considers UTXO's that are:
* * CONFIRMED
* * NOT RESERVED
* * Greater in value than the fee they would cause (using 70 bytes for input size).
* Consolidator will leave the smallest available UTXO that is greater than CLN's ``min-emergency-msat`` value untouched

# Options

* ``consolidator-interval`` *interval_secs*
* * Interval the background task from ``consolidate-below`` uses to check the feerate. Defaults to 3600 (1 hour)