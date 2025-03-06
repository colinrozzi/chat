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

# 5. Update the state_trait.rs to use the runtime store state
echo "Updating state_trait.rs to use runtime_store state"
cat <<EOF > src/state_trait.rs
// This module provides a common interface for both state implementations

// Re-export the State type from the current state implementation
pub use crate::state_runtime_store::State;
EOF

echo "Done! The chat actor now uses the Theater runtime store."
echo "To build and run: cargo build --target wasm32-unknown-unknown"
