#!/bin/bash

# Script to switch to the runtime store implementation

echo "Switching Chat Actor to use the Theater Runtime Store..."

# 1. Back up the current lib.rs file
echo "Backing up current lib.rs to lib_store_actor.rs"
cp src/lib.rs src/lib_store_actor.rs

# 2. Back up the current state.rs file
echo "Backing up current state.rs to state_store_actor.rs"
cp src/state.rs src/state_store_actor.rs

# 3. Replace lib.rs with the runtime store version
echo "Installing runtime store implementation of lib.rs"
cp src/lib_runtime_store.rs src/lib.rs

# 4. Replace state.rs with the runtime store version
echo "Installing runtime store implementation of state.rs"
cp src/state_runtime_store.rs src/state.rs

echo "Switching to use external store module instead of default store module"
cat <<EOF > src/messages/switch_store.rs
// This file redirects imports from messages::store to messages::runtime_store
pub use crate::messages::runtime_store::*;
EOF

echo "Done! The chat actor now uses the Theater runtime store."
echo "To build and run: cargo build --target wasm32-unknown-unknown"
