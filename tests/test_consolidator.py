#!/usr/bin/python


import pytest
from pyln.client import RpcError
from pyln.testing.fixtures import *  # noqa: F403
from pyln.testing.utils import sync_blockheight, wait_for
from util import get_plugin  # noqa: F401


def test_basic(node_factory, bitcoind, get_plugin):  # noqa: F811
    l1 = node_factory.get_node(options={"plugin": get_plugin})
    l1.fundwallet(100_000)
    l1.fundwallet(100_000)
    l1.fundwallet(20_000)
    l1.fundwallet(50_000)
    l1.fundwallet(590)
    l1.fundwallet(100_000)
    l1.fundwallet(100_000)

    with pytest.raises(
        RpcError, match=r"Feerate 2000perkb is below min_acceptable of 7500perkb"
    ):
        l1.rpc.call("consolidate", {"feerate": 2000, "min_utxos": 5})

    with pytest.raises(RpcError, match=r"Not enough UTXO's to consolidate: 5<10"):
        l1.rpc.call("consolidate", {"feerate": 8500, "min_utxos": 10})

    result = l1.rpc.call("consolidate", {"feerate": 8500, "min_utxos": 5})

    assert result["num_utxos_consolidating"] == 5

    tx = bitcoind.rpc.decoderawtransaction(result["tx"])
    assert float(tx["vout"][0]["value"]) * 100000000 > (420000 - tx["vsize"] * 2) * 0.99
    assert float(tx["vout"][0]["value"]) * 100000000 < (420000 - tx["vsize"] * 2) * 1.01

    bitcoind.generate_block(1, wait_for_mempool=1)
    sync_blockheight(bitcoind, [l1])

    result = l1.rpc.call("consolidate", {"feerate": 7500, "min_utxos": 2})

    assert result["num_utxos_consolidating"] == 2

    tx = bitcoind.rpc.decoderawtransaction(result["tx"])
    assert float(tx["vout"][0]["value"]) * 100000000 > (420000 - tx["vsize"] * 2) * 0.99
    assert float(tx["vout"][0]["value"]) * 100000000 < (420000 - tx["vsize"] * 2) * 1.01


def test_below(node_factory, bitcoind, get_plugin):  # noqa: F811
    l1 = node_factory.get_node(
        options={"plugin": get_plugin, "consolidator-interval": 2}
    )
    l1.fundwallet(100_000)
    l1.fundwallet(100_000)
    l1.fundwallet(20_000)
    l1.fundwallet(50_000)
    l1.fundwallet(590)
    l1.fundwallet(100_000)
    l1.fundwallet(100_000)

    result = l1.rpc.call("consolidate-below", {"feerate": 8000, "min_utxos": 5})

    assert result["result"] == "OK"

    wait_for(lambda: l1.daemon.is_in_log(r"Feerate not low enough yet: 30000<8000"))

    with pytest.raises(RpcError, match=r"Already have a consolidate-below running!"):
        l1.rpc.call("consolidate-below", {"feerate": 31000, "min_utxos": 5})

    result = l1.rpc.call("consolidate-cancel", {})
    assert result["result"] == "Canceled"
    wait_for(lambda: l1.daemon.is_in_log(r"consolidate_below CANCELED"))

    result = l1.rpc.call("consolidate-below", {"feerate": 31000, "min_utxos": 5})
    assert result["result"] == "OK"
    wait_for(lambda: l1.daemon.is_in_log(r"consolidate_below: SUCCESS:"))