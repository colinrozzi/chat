# Change Request: Add JavaScript Bundling for Frontend

## Problem Statement

Currently, the frontend code is organized as a single large JavaScript file (`chat.js`), which makes it difficult to maintain and scale. As the application grows, this will lead to increased complexity, code duplication, and potential conflicts during development.

## Proposed Solution

Implement a JavaScript bundling system using esbuild integrated with our existing Nix build pipeline. This approach will allow us to:

1. Split code into modular components
2. Organize the frontend codebase into a more maintainable structure
3. Bundle everything into optimized production files
4. Keep the build process deterministic through Nix

## Implementation Plan

### 1. Create a New Frontend Directory Structure

```
assets/
├── src/
│   ├── index.js           # Main entry point
│   ├── components/        # UI components
│   │   ├── chat.js        # Chat UI logic
│   │   ├── message.js     # Message component
│   │   └── sidebar.js     # Chat list sidebar
│   ├── services/          # Services
│   │   ├── api.js         # API interaction
│   │   └── websocket.js   # WebSocket handling
│   └── utils/             # Utilities
│       ├── formatters.js  # Message formatting
│       └── storage.js     # Client-side storage
├── dist/                  # Bundled output
│   └── chat.js            # Final bundled JS (output)
├── index.html             # HTML entry point
└── styles.css             # Styles
```

### 2. Update Nix Configuration

Modify `flake.nix` to include esbuild and configure the bundling process:

```nix
# Add to inputs if necessary
inputs = {
  nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  # existing inputs...
};

# Update the buildPhase to include esbuild
buildPhase = ''
  # Bundle JavaScript
  mkdir -p assets/dist
  ${pkgs.esbuild}/bin/esbuild \
    assets/src/index.js \
    --bundle \
    --minify \
    --outfile=assets/dist/chat.js
    
  # Build the Rust component
  cargo component build --release --target wasm32-unknown-unknown
'';

# Add esbuild to nativeBuildInputs
nativeBuildInputs = with pkgs; [
  pkg-config
  rustc
  cargo
  cargo-component
  esbuild
  # Other build dependencies
];
```

### 3. Split Current JavaScript File

1. Analyze the current `chat.js` file to identify logical components
2. Create new modular files in the appropriate directories
3. Use ES modules syntax for imports/exports
4. Update references in HTML to point to the bundled output

### 4. Update HTTP Handler

Modify the HTTP handler in `src/handlers/http.rs` to serve files from the `assets/dist` directory for JavaScript resources.

### 5. Development Workflow

Add a development script to allow for faster iteration during development:

```nix
# Add to flake.nix devShell
devShell = forAllSystems (system:
  let
    pkgs = nixpkgs.legacyPackages.${system};
  in
  pkgs.mkShell {
    nativeBuildInputs = with pkgs; [
      # existing tools...
      esbuild
    ];
    
    shellHook = ''
      # Add a dev script
      function dev-frontend() {
        ${pkgs.esbuild}/bin/esbuild \
          --bundle assets/src/index.js \
          --outfile=assets/dist/chat.js \
          --servedir=assets \
          --serve=0.0.0.0:8085
      }
      
      echo "Run 'dev-frontend' to start the frontend development server"
    '';
  }
);
```

## Expected Benefits

1. **Improved Code Organization**: Clear separation of concerns
2. **Better Maintainability**: Smaller, focused files
3. **Enhanced Development Experience**: Faster build times with esbuild
4. **Future-Proofing**: Easier to add new features like TypeScript if desired
5. **Reduced File Size**: Minification for production

## Testing Plan

1. Verify that all existing functionality works after the changes
2. Test the development workflow
3. Compare bundle size and performance with the current implementation
4. Ensure compatibility across browsers

## Time Estimate

- 1 day for Nix configuration and build setup
- 2 days for refactoring the JavaScript code
- 1 day for testing and final adjustments

## Dependencies

- esbuild (via Nix)
- Updates to HTTP handlers

## Notes

The initial migration should maintain feature parity with the current implementation. Once this foundation is in place, we can consider further enhancements like:

1. Adding TypeScript support
2. Implementing CSS preprocessing
3. Adding unit tests for UI components
