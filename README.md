# Changes Needed for kryon-renderer README

## 1. Update the Architecture Diagram
Replace the existing diagram section with:

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   kryon-core    │    │  kryon-layout   │    │  kryon-render   │
│                 │    │                 │    │                 │
│ • KRB Parser    │    │ • Layout Engine │    │ • Render Traits │
│ • Element Types │    │ • Flexbox       │    │ • Commands      │
│ • Properties    │    │ • Constraints   │    │ • Events        │
└─────────────────┘    └─────────────────┘    └─────────────────┘
         │                       │                       │
         └───────────────────────┼───────────────────────┘
                                 │
              ┌─────────────────────────────────┐
              │        kryon-runtime            │
              │                                 │
              │ • App Management               │
              │ • Event System                 │
              │ • Script Integration           │
              │ • Backend Selection            │
              └─────────────────────────────────┘
                                 │
                    ┌────────────┼────────────┐
                    │            │            │
         ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐
         │   kryon-wgpu    │  │ kryon-ratatui   │  │  kryon-raylib   │
         │                 │  │                 │  │                 │
         │ • GPU Rendering │  │ • Terminal UI   │  │ • 2D/3D Graphics│
         │ • Cross-platform│  │ • Text Mode     │  │ • Game-focused  │
         │ • High Performance  │ • ASCII Graphics│  │ • Easy Setup    │
         └─────────────────┘  └─────────────────┘  └─────────────────┘
```

## 2. Update Features Section
Add raylib to the backend list:

- **WGPU Backend**: High-performance GPU rendering for desktop/mobile/web
- **Ratatui Backend**: Terminal-based UI for CLI applications
- **Raylib Backend**: Simple 2D/3D graphics for games and multimedia apps

## 3. Update Building Instructions
```bash
# Build all backends
cargo build

# Build specific backend only
cargo build --no-default-features --features wgpu     # GPU rendering
cargo build --no-default-features --features ratatui  # Terminal rendering
cargo build --no-default-features --features raylib   # Raylib rendering
```

## 4. Update Usage Examples
```bash
# Render with WGPU (default)
./target/debug/kryon-renderer examples/hello_world.krb

# Render with terminal backend
./target/debug/kryon-renderer examples/hello_world.krb --backend terminal

# Render with raylib backend
./target/debug/kryon-renderer-raylib examples/hello_world.krb
# OR (if you want consistent naming)
./target/debug/kryon-renderer examples/hello_world.krb --backend raylib

# Show available backends and options
./target/debug/kryon-renderer --info
```

## 5. Update Backend Selection Section
Choose your rendering backend based on your needs:

- **WGPU**: Best for desktop apps, games, and high-performance UIs
- **Ratatui**: Perfect for CLI tools, server applications, and terminal UIs  
- **Raylib**: Great for simple games, prototypes, and learning graphics programming

## 6. Update Building Section
```bash
# Build all crates
cargo build --workspace

# Build with specific features
cargo build --features wgpu
cargo build --features ratatui
cargo build --features raylib

# Build examples
chmod +x build.sh
./build.sh
```

## 7. Consider Adding Default Features
If you want raylib to be enabled by default, add this to your Cargo.toml:

```toml
[features]
default = ["raylib"]
raylib = ["dep:raylib"]
wgpu = ["dep:wgpu"]
ratatui = ["dep:ratatui"]
```

## Development & Testing

This project uses snapshot testing to ensure the visual correctness of the UI renderer. Due to the deterministic nature of terminal output, the `kryon-ratatui` backend serves as the primary target for these tests. This provides a stable and reliable way to verify rendering logic without the complexities of image comparison.

### Snapshot Testing Workflow

The testing process is managed by the `insta` snapshot testing library.

1.  **Run the tests** for the `ratatui` backend:
    ```bash
    cargo test -p kryon-ratatui
    ```

2.  **Initial Failure**: The first time a new test is run, or after an intentional visual change, the test will fail. This is expected. `insta` will save the new output to a `.snap.new` file.

3.  **Review Changes**: To review the new snapshot and see a diff of the changes, run:
    ```bash
    cargo insta review
    ```

4.  **Accept or Reject**: An interactive tool will open in your terminal.
    - Press `a` to **a**ccept the new snapshot if the changes are correct.
    - Press `r` to **r**eject the changes if they introduce a bug.

5.  **Verify**: After accepting a snapshot, run the test again to confirm that it passes.