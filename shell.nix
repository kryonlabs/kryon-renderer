# A robust Nix shell for Rust graphics development (WGPU, Raylib, etc.)
{ pkgs ? import <nixpkgs> {} }:

pkgs.mkShell {
  # nativeBuildInputs are packages needed to *build* your code.
  # This includes compilers, build tools, headers (.dev packages), and pkg-config files.
  # mkShell automatically uses these to set up PKG_CONFIG_PATH and other env vars.
  nativeBuildInputs = with pkgs; [
    # 1. Rust Toolchain
    # Provides cargo, rustc, etc. Also includes recommended tools.
    rustc
    cargo
    rust-analyzer
    clippy
    rustfmt

    # 2. Core Build Tools
    # Essential for building C/C++ dependencies like raylib's or winit's.
    pkg-config
    cmake
    gcc

    # 3. Dependencies for `rust-bindgen`
    # The raylib-sys crate uses bindgen, which needs libclang.
    llvmPackages.libclang # Provides the libclang.so library
    llvmPackages.bintools # Provides ld, as, etc.

    # 4. System Libraries (Development versions)
    # The .dev output includes headers and is what build systems look for.
    glibc.dev # Standard C library headers

    # 5. Graphics & Windowing System Libraries
    # For both WGPU (Vulkan/Wayland/X11) and Raylib (OpenGL/ALSA/X11)

    # Vulkan for WGPU
    vulkan-loader
    vulkan-validation-layers
    vulkan-tools # Adds the useful `vulkaninfo` command for debugging

    # OpenGL Libraries (CRITICAL for raylib/GLFW)
    libGL.dev          # OpenGL library headers
    libglvnd.dev       # Vendor-neutral GL dispatch library (FIXES GLX ERRORS)
    mesa.dev           # Mesa 3D graphics library
    mesa.drivers       # Mesa drivers for software/hardware rendering

    # X11 Libraries (needed by both winit and raylib/GLFW)
    xorg.libX11.dev
    xorg.libXcursor.dev
    xorg.libXrandr.dev
    xorg.libXi.dev
    xorg.libXinerama.dev
    xorg.libXfixes.dev
    xorg.libXrender.dev
    xorg.libXext.dev
    xorg.libXxf86vm.dev    # Required by GLFW for video mode switching
    xorg.libXmu.dev        # X11 miscellaneous utilities
    xorg.libXpm.dev        # X11 pixmap library

    # Wayland Libraries (needed by winit for wgpu)
    wayland.dev
    libxkbcommon.dev       # Base xkbcommon library
    xorg.libxkbfile.dev    # X11 xkb file handling

    # Audio (for raylib)
    alsa-lib.dev
    pulseaudio.dev     # PulseAudio support
  ];

  # buildInputs are runtime libraries that need to be available
  # when running the compiled programs
  buildInputs = with pkgs; [
    # Runtime OpenGL libraries
    libGL
    libglvnd
    mesa.drivers

    # Runtime X11 libraries  
    xorg.libX11
    xorg.libXcursor
    xorg.libXrandr
    xorg.libXi
    xorg.libXinerama
    xorg.libXfixes
    xorg.libXrender
    xorg.libXext
    xorg.libXxf86vm

    # Wayland/XKB runtime (CRITICAL for WGPU)
    wayland
    libxkbcommon        # Base xkbcommon runtime - includes X11 support
    xorg.libxkbfile     # X11 xkb file handling runtime

    # Audio runtime
    alsa-lib
    pulseaudio
  ];

  # Environment variables needed for specific tools that can't find
  # their dependencies automatically.

  # Tell the Vulkan loader where to find the validation layers.
  VK_LAYER_PATH = "${pkgs.vulkan-validation-layers}/share/vulkan/explicit_layer.d";

  # Tell `rust-bindgen` exactly where to find the libclang.so file.
  LIBCLANG_PATH = "${pkgs.llvmPackages.libclang.lib}/lib";

  # Give `bindgen` extra hints to find system headers in Nix's non-standard paths.
  BINDGEN_EXTRA_CLANG_ARGS = "-I${pkgs.glibc.dev}/include -I${pkgs.gcc.cc}/lib/gcc/${pkgs.stdenv.hostPlatform.config}/${pkgs.gcc.cc.version}/include";

  # OpenGL/GLX Environment Variables (CRITICAL for raylib)
  # These tell the system where to find the OpenGL libraries
  LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath [
    pkgs.libGL
    pkgs.libglvnd
    pkgs.mesa.drivers
    pkgs.xorg.libX11
    pkgs.xorg.libXcursor
    pkgs.xorg.libXrandr
    pkgs.xorg.libXi
    pkgs.xorg.libXinerama
    pkgs.xorg.libXfixes
    pkgs.xorg.libXxf86vm
    pkgs.wayland
    pkgs.libxkbcommon      # CRITICAL for WGPU X11 support
    pkgs.xorg.libxkbfile   # Additional XKB support
    pkgs.alsa-lib
    pkgs.pulseaudio
  ];

  # Ensure mesa drivers are found
  LIBGL_DRIVERS_PATH = "${pkgs.mesa.drivers}/lib/dri";
  
  # Help GLFW find the right display
  DISPLAY = ":0"; # Default X11 display (adjust if needed)

  # The shellHook runs commands every time you enter the shell.
  shellHook = ''
    # Add a welcome message
    echo "Kryon Renderer development environment is ready."
    echo "  - Rust, WGPU, and Raylib dependencies are loaded."
    echo "  - OpenGL/GLX libraries configured for raylib."
    echo "  - XKB libraries configured for WGPU."
    echo "  - For Vulkan diagnostics, run: vulkaninfo"
    
    # Check if we're in a graphical environment
    if [ -z "$DISPLAY" ] && [ -z "$WAYLAND_DISPLAY" ]; then
      echo ""
      echo "⚠️  WARNING: No display detected. You may need to:"
      echo "   - Run 'export DISPLAY=:0' for X11"
      echo "   - Or run from a graphical terminal"
    fi
    
    # Test OpenGL availability
    if command -v glxinfo >/dev/null 2>&1; then
      echo ""
      echo "OpenGL Status:"
      glxinfo | grep "OpenGL version" || echo "  - Could not detect OpenGL version"
    else
      echo ""
      echo "💡 Install glxinfo for OpenGL diagnostics: nix-shell -p glxinfo"
    fi
    
    # Test XKB availability for WGPU
    echo ""
    echo "XKB Libraries:"
    if [ -f "${pkgs.libxkbcommon}/lib/libxkbcommon.so" ]; then
      echo "  ✓ libxkbcommon.so found"
    else
      echo "  ✗ libxkbcommon.so missing"
    fi
    if [ -f "${pkgs.libxkbcommon}/lib/libxkbcommon-x11.so" ]; then
      echo "  ✓ libxkbcommon-x11.so found"
    else
      echo "  ✗ libxkbcommon-x11.so missing"
    fi
    
    # Unset RUST_LOG to prevent spam from wgpu/naga in the shell.
    # If you need to debug wgpu, you can set it manually:
    #   export RUST_LOG=wgpu=debug
    unset RUST_LOG
    
    echo ""
    echo "Available binaries:"
    echo "  cargo run --bin kryon-renderer-raylib --features raylib examples/hello_world.krb"
    echo "  cargo run --bin kryon-renderer-wgpu --features wgpu examples/hello_world.krb"
    echo "  cargo run --bin kryon-renderer-ratatui --features ratatui examples/hello_world.krb"
    echo "  cargo run examples/hello_world.krb  # Uses default backend (WGPU)"
    echo ""
    echo "Quick tests:"
    echo "  cargo run --bin kryon-renderer-raylib --features raylib examples/hello_world.krb"
    echo "  cargo run examples/hello_world.krb"
  '';
}