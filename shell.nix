# A robust Nix shell for Rust graphics development (WGPU, Raylib, etc.)
{ pkgs ? import <nixpkgs> {} }:

pkgs.mkShell {
  nativeBuildInputs = with pkgs; [
    # 1. Rust Toolchain
    rustc
    cargo
    rust-analyzer
    clippy
    rustfmt

    # 2. Core Build Tools
    pkg-config
    cmake
    gcc

    # 3. Dependencies for `rust-bindgen`
    llvmPackages.libclang
    llvmPackages.bintools

    # 4. System Libraries (Development versions)
    glibc.dev

    # 5. Graphics & Windowing System Libraries

    # Vulkan for WGPU (CRITICAL)
    vulkan-loader
    vulkan-validation-layers
    vulkan-tools
    vulkan-headers
    spirv-tools
    shaderc

    # OpenGL Libraries (for raylib and fallback)
    libGL.dev
    libglvnd.dev
    mesa.dev
    mesa.drivers

    # Intel Graphics (CRITICAL for your GPU)
    intel-media-driver      # Intel media driver
    libva                   # Video acceleration
    libva-utils             # VA-API utilities

    # X11 Libraries
    xorg.libX11.dev
    xorg.libXcursor.dev
    xorg.libXrandr.dev
    xorg.libXi.dev
    xorg.libXinerama.dev
    xorg.libXfixes.dev
    xorg.libXrender.dev
    xorg.libXext.dev
    xorg.libXxf86vm.dev
    xorg.libXmu.dev
    xorg.libXpm.dev

    # Wayland Libraries
    wayland.dev
    libxkbcommon.dev
    xorg.libxkbfile.dev

    # Audio
    alsa-lib.dev
    pulseaudio.dev
  ];

  buildInputs = with pkgs; [
    # Runtime Vulkan (CRITICAL for WGPU)
    vulkan-loader
    vulkan-validation-layers
    mesa.drivers

    # Intel Graphics Runtime (CRITICAL for your Intel HD 620)
    intel-media-driver
    libva
    
    # Runtime OpenGL libraries
    libGL
    libglvnd
    
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

    # Wayland/XKB runtime
    wayland
    libxkbcommon
    xorg.libxkbfile

    # Audio runtime
    alsa-lib
    pulseaudio
  ];

  # Environment variables
  VK_LAYER_PATH = "${pkgs.vulkan-validation-layers}/share/vulkan/explicit_layer.d";
  LIBCLANG_PATH = "${pkgs.llvmPackages.libclang.lib}/lib";
  BINDGEN_EXTRA_CLANG_ARGS = "-I${pkgs.glibc.dev}/include -I${pkgs.gcc.cc}/lib/gcc/${pkgs.stdenv.hostPlatform.config}/${pkgs.gcc.cc.version}/include";

  # LD_LIBRARY_PATH for runtime libraries
  LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath [
    # Vulkan runtime (ESSENTIAL for WGPU)
    pkgs.vulkan-loader
    pkgs.mesa.drivers

    # Intel graphics
    pkgs.intel-media-driver
    pkgs.libva

    # OpenGL
    pkgs.libGL
    pkgs.libglvnd

    # X11 libraries
    pkgs.xorg.libX11
    pkgs.xorg.libXcursor
    pkgs.xorg.libXrandr
    pkgs.xorg.libXi
    pkgs.xorg.libXinerama
    pkgs.xorg.libXfixes
    pkgs.xorg.libXxf86vm

    # Wayland/XKB
    pkgs.wayland
    pkgs.libxkbcommon
    pkgs.xorg.libxkbfile

    # Audio
    pkgs.alsa-lib
    pkgs.pulseaudio
  ];

  # CRITICAL: Vulkan ICD files for Intel (FIXED PATH)
  VK_ICD_FILENAMES = "${pkgs.mesa.drivers}/share/vulkan/icd.d/intel_icd.x86_64.json";
  
  # Mesa driver paths
  LIBGL_DRIVERS_PATH = "${pkgs.mesa.drivers}/lib/dri";
  LIBVA_DRIVERS_PATH = "${pkgs.intel-media-driver}/lib/dri";
  
  # Force Intel graphics
  MESA_LOADER_DRIVER_OVERRIDE = "i965";  # or "iris" for newer Intel
  
  # Help with display
  DISPLAY = ":0";

  # WGPU specific debugging (CRITICAL)
  WGPU_BACKEND = "vulkan";  # Force Vulkan backend
  WGPU_POWER_PREF = "low";  # Prefer integrated GPU

  shellHook = ''
    unset RUST_LOG
    
    # Quick renderer aliases
    alias kryon-raylib='cargo run --features raylib --bin kryon-renderer-raylib --'
    alias kryon-wgpu='cargo run --features wgpu --bin kryon-renderer-wgpu --'
    alias kryon-ratatui='RUST_LOG=off cargo run --features ratatui --bin kryon-renderer-ratatui --'
    alias kryon-debug='cargo run --bin kryon-renderer-debug --'
    
    # For ratatui with log file redirection
    alias kryon-ratatui-log='RUST_LOG=debug cargo run --features ratatui --bin kryon-renderer-ratatui -- 2>ratatui.log'
    
    echo "🚀 QUICK ALIASES:"
    echo ""
    echo "  kryon-raylib <file.krb>     - Run with Raylib backend"
    echo "  kryon-wgpu <file.krb>       - Run with WGPU backend"
    echo "  kryon-ratatui <file.krb>    - Run with Ratatui backend (no logs)"
    echo "  kryon-ratatui-log <file.krb>- Run with Ratatui backend (logs to ratatui.log)"
    echo "  kryon-debug <file.krb>      - Run with debug backend"
    echo ""
    echo "📝 EXAMPLES:"
    echo "  kryon-raylib examples/02_basic_ui/tabbar_bottom_demo.krb"
    echo "  kryon-ratatui examples/01_getting_started/hello_world.krb"
    echo ""
  '';
}