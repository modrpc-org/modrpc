#!/bin/sh
cargo build --release --target wasm32-unknown-unknown
wasm-tools component new \
    target/wasm32-unknown-unknown/release/INTERFACE_NAME_ROLE_NAME_modrpc_wasm.wasm \
    -o INTERFACE_NAME_ROLE_NAME_modrpc.wasm
