// crates/kryon-runtime/src/backends.rs
#[cfg(feature = "wgpu")]
pub use kryon_wgpu::WgpuRenderer;

#[cfg(feature = "ratatui")]
pub use kryon_ratatui::RatatuiRenderer;

/// Backend selection enum
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RendererBackend {
    #[cfg(feature = "wgpu")]
    Wgpu,
    #[cfg(feature = "ratatui")]
    Ratatui,
}

impl RendererBackend {
    pub fn name(&self) -> &'static str {
        match self {
            #[cfg(feature = "wgpu")]
            RendererBackend::Wgpu => "wgpu",
            #[cfg(feature = "ratatui")]
            RendererBackend::Ratatui => "ratatui",
        }
    }
    
    pub fn available_backends() -> Vec<RendererBackend> {
        let mut backends = Vec::new();
        
        #[cfg(feature = "wgpu")]
        backends.push(RendererBackend::Wgpu);
        
        #[cfg(feature = "ratatui")]
        backends.push(RendererBackend::Ratatui);
        
        backends
    }
}