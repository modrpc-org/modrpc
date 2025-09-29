#!/bin/sh
npm install .
npx jco transpile ../rust/INTERFACE_NAME_ROLE_NAME_modrpc.wasm -o dist/wasm/ --instantiation
tsc
