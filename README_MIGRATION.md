# Chat Actor Migration to Theater Runtime Store

This directory contains an implementation of the Chat Actor that uses the Theater runtime's built-in content-addressed store system instead of the standalone store actor.

## Quick Start

Two scripts are provided to easily switch between implementations:

### Switch to Runtime Store

```bash
chmod +x switch_to_runtime_store.sh
./switch_to_runtime_store.sh
```

### Switch Back to Store Actor

```bash
chmod +x switch_to_store_actor.sh
./switch_to_store_actor.sh
```

After switching, rebuild the actor:

```bash
cargo build --target wasm32-unknown-unknown --release
```

## Implementation Details

This implementation replaces the standalone store actor with the Theater runtime's built-in content-addressed store. Key benefits include:

1. **Simplified Architecture**: No need for a separate store actor
2. **Direct API Access**: More efficient than message-passing
3. **Enhanced Features**: Content labeling, deduplication, and better navigation

See the full migration guide for detailed technical information.

## Files

- `src/messages/runtime_store.rs`: New MessageStore implementation using runtime store
- `src/state_runtime_store.rs`: State implementation for runtime store
- `src/lib_runtime_store.rs`: Main lib.rs using runtime store
- `switch_to_runtime_store.sh`: Script to switch to runtime store
- `switch_to_store_actor.sh`: Script to revert to store actor

## Notes

- The runtime store implementation is designed to be a drop-in replacement
- No automatic data migration is provided between implementations
- Both implementations maintain the same API and behavior
