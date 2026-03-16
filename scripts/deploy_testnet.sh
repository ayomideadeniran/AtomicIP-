#!/usr/bin/env bash
set -e
source .env

stellar keys generate deployer --network testnet

stellar contract deploy \
  --wasm target/wasm32-unknown-unknown/release/ip_registry.wasm \
  --source deployer \
  --network testnet

stellar contract deploy \
  --wasm target/wasm32-unknown-unknown/release/atomic_swap.wasm \
  --source deployer \
  --network testnet
