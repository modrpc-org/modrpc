#!/bin/sh

set -e

# Change to this script's directory
cd $(dirname -- "$( readlink -f -- "$0"; )")

cd ../crates/modrpcc/
cargo build --release
cd -

../target/release/modrpcc proto/foo.modrpc -l rust -o . -n foo

# Attempt to compile rust package
cd foo-modrpc/rust/
cargo build
cd -

echo "modrpc integ tests passed."
