#!/bin/sh

set -e

# Change to this script's directory
cd $(dirname -- "$( readlink -f -- "$0"; )")

RUST_BACKTRACE=1 ../target/release/modrpcc ../proto/std.modrpc -l rust -o ../ -n std
