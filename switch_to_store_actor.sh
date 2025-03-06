#!/bin/bash

# Script to switch back to the store actor implementation

echo "Switching Chat Actor to use the Store Actor..."

# 1. Check if backup files exist
if [ ! -f src/lib_store_actor.rs ]; then
    echo "Error: Backup file src/lib_store_actor.rs not found. Cannot switch back."
    exit 1
fi

if [ ! -f src/state_store_actor.rs ]; then
    echo "Error: Backup file src/state_store_actor.rs not found. Cannot switch back."
    exit 1
fi

# 2. Restore the lib.rs file
echo "Restoring original lib.rs implementation"
cp src/lib_store_actor.rs src/lib.rs

# 3. Restore the state.rs file
echo "Restoring original state.rs implementation"
cp src/state_store_actor.rs src/state.rs

# 4. Remove any switch_store.rs file if it exists
if [ -f src/messages/switch_store.rs ]; then
    echo "Removing switch_store.rs"
    rm src/messages/switch_store.rs
fi

echo "Done! The chat actor now uses the original store actor."
echo "To build and run: cargo build --target wasm32-unknown-unknown"
