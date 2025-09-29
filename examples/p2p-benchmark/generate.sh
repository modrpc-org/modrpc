#!/bin/sh

set -e

# Change to this script's directory
cd $(dirname -- "$( readlink -f -- "$0"; )")

../../target/release/modrpcc -l rust -n p2p-benchmark p2p-benchmark.modrpc
