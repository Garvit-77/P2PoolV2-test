#!/bin/sh

bitcoind -conf=/bitcoin.conf -daemon
until bitcoin-cli -conf=/bitcoin.conf -regtest getblockchaininfo > /dev/null 2>&1; do
  sleep 5
done
bitcoin-cli -conf=/bitcoin.conf -regtest -named createwallet wallet_name=mywallet descriptors=false
rust-bitcoin-tx
tail -f /dev/null