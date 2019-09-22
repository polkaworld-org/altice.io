#!/bin/zsh

set -e

./scripts/build.sh

rm Cargo.lock
cp Cargo.lock.good Cargo.lock
cargo build

rm -rf /tmp/alice

./target/debug/daqiao --chain local --port 30333 --base-path /tmp/alice --node-key 0000000000000000000000000000000000000000000000000000000000000001 --telemetry-url ws://telemetry.polkadot.io:1024 --validator --key //Alice --force-authoring
