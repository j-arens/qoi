#!/usr/bin/env sh

set -e

echo
echo "==> downloading and running rustup"
echo

# Download and run the rustup-init.sh script:
# `-y` to disable the confirmation prompt
# `-t wasm32-unknown-unknown` to install the `wasm32-unknown-unknown` target
curl https://sh.rustup.rs -sSf | sh -s -- -y -t wasm32-unknown-unknown

# Refresh the shell and add cargo to `PATH`.
source $HOME/.cargo/env

# Verify cargo is working.
cargo -V

# Print installed targets for potential debugging purposes.
echo
echo "==> printing rustc target list"
echo

rustc --print target-list

echo
echo "==> running build"
echo

# Run the `site_util` build command.
cd ./site_util
cargo run -- build

echo
echo "==> build complete"
echo
