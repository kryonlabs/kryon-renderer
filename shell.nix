{ pkgs ? import <nixpkgs> {} }:

pkgs.mkShell {
  buildInputs = with pkgs; [
    # Rust toolchain
    rustc
    cargo
    rust-analyzer
    clippy
    rustfmt
    
    # Graphics libraries for WGPU
    vulkan-loader
    vulkan-validation-layers
    vulkan-tools
    
    # X11/Wayland support
    xorg.libX11
    xorg.libXcursor
    xorg.libXrandr
    xorg.libXi
    libxkbcommon
    wayland
    
    # OpenGL support
    libGL
    mesa
    
    # Audio (for potential future use)
    alsa-lib
    
    # Development tools
    pkg-config
    cmake
    
    # Terminal support
    ncurses
  ];
  
  # Environment variables for graphics
  LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath [
    pkgs.vulkan-loader
    pkgs.libGL
    pkgs.mesa
    pkgs.xorg.libX11
    pkgs.xorg.libXcursor
    pkgs.xorg.libXrandr
    pkgs.xorg.libXi
    pkgs.libxkbcommon
    pkgs.wayland
    pkgs.alsa-lib
  ];
  
  # Vulkan environment
  VK_LAYER_PATH = "${pkgs.vulkan-validation-layers}/share/vulkan/explicit_layer.d";
  
  shellHook = ''
    echo "Kryon Renderer development environment"
    echo "Graphics libraries and Vulkan support loaded"
    echo ""
    echo "Available binaries:"
    echo "  cargo run -- <file.krb>                    # Full renderer (default: WGPU)"
    echo "  cargo run -- <file.krb> --backend terminal # Terminal renderer" 
    echo "  cargo run --bin kryon-render-terminal <file.krb>  # Simple parser"
    echo ""
    echo "Example:"
    echo "  cargo run -- examples/hello_world.krb"
  '';
}