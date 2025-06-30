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
  
  # Alternative: Let Vulkan find all drivers automatically
  # VK_DRIVER_FILES = "${pkgs.mesa.drivers}/share/vulkan/icd.d/intel_icd.x86_64.json";
  
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
    echo "Kryon Renderer development environment ready (Intel HD Graphics 620 optimized)"
    
    # Check environment
    echo ""
    echo "🔍 Graphics Environment:"
    
    # Check ICD files
    if [ -f "${pkgs.mesa.drivers}/share/vulkan/icd.d/intel_icd.x86_64.json" ]; then
      echo "  ✓ Intel Vulkan ICD found"
    else
      echo "  ⚠️  Intel Vulkan ICD missing"
      echo "  📁 Looking in: ${pkgs.mesa.drivers}/share/vulkan/icd.d/"
      ls -la "${pkgs.mesa.drivers}/share/vulkan/icd.d/" 2>/dev/null || echo "    Directory not found"
    fi
    
    # Check Vulkan
    if vulkaninfo --summary 2>/dev/null | grep -q "Intel"; then
      echo "  ✓ Intel GPU detected via Vulkan"
    else
      echo "  ⚠️  Intel GPU not detected via Vulkan"
    fi
    
    # Check OpenGL
    if glxinfo 2>/dev/null | grep -q "Intel"; then
      echo "  ✓ Intel GPU detected via OpenGL"
    else
      echo "  ⚠️  Install glxinfo: nix-shell -p glxinfo"
    fi
    
    unset RUST_LOG
    
    echo ""
    echo "🚀 Test commands:"
    echo "  # Debug WGPU adapter detection:"
    echo "  RUST_LOG=wgpu=debug cargo run --bin kryon-renderer-wgpu --features wgpu examples/hello_world.krb"
    echo ""
    echo "  # Other backends:"
    echo "  cargo run --bin kryon-renderer-raylib --features raylib examples/hello_world.krb"
    echo "  cargo run --bin kryon-renderer-ratatui --features ratatui examples/hello_world.krb"
  '';
}