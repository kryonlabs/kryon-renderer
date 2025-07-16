//! Web asset loading for fonts, images, and other resources

use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::{Request, RequestInit, RequestMode, Response};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub enum Asset {
    Image(Vec<u8>),
    Font(Vec<u8>),
    Audio(Vec<u8>),
    Binary(Vec<u8>),
}

pub struct WebAssetLoader {
    cache: HashMap<String, Asset>,
    loading: HashMap<String, JsFuture>,
}

impl WebAssetLoader {
    pub fn new() -> Self {
        Self {
            cache: HashMap::new(),
            loading: HashMap::new(),
        }
    }
    
    pub async fn load_asset(&mut self, url: &str) -> Result<Asset, JsValue> {
        // Check cache first
        if let Some(asset) = self.cache.get(url) {
            return Ok(asset.clone());
        }
        
        // Load from network
        let asset = self.fetch_asset(url).await?;
        self.cache.insert(url.to_string(), asset.clone());
        Ok(asset)
    }
    
    pub async fn load_image(&mut self, url: &str) -> Result<Vec<u8>, JsValue> {
        match self.load_asset(url).await? {
            Asset::Image(data) => Ok(data),
            asset => {
                // Try to convert other asset types to image
                match asset {
                    Asset::Binary(data) => Ok(data),
                    _ => Err(JsValue::from_str("Asset is not an image")),
                }
            }
        }
    }
    
    pub async fn load_font(&mut self, url: &str) -> Result<Vec<u8>, JsValue> {
        match self.load_asset(url).await? {
            Asset::Font(data) => Ok(data),
            asset => {
                // Try to convert other asset types to font
                match asset {
                    Asset::Binary(data) => Ok(data),
                    _ => Err(JsValue::from_str("Asset is not a font")),
                }
            }
        }
    }
    
    pub async fn load_krb(&mut self, url: &str) -> Result<Vec<u8>, JsValue> {
        match self.load_asset(url).await? {
            Asset::Binary(data) => Ok(data),
            _ => Err(JsValue::from_str("Asset is not binary data")),
        }
    }
    
    async fn fetch_asset(&self, url: &str) -> Result<Asset, JsValue> {
        let window = web_sys::window().ok_or("No window object")?;
        
        let mut opts = RequestInit::new();
        opts.method("GET");
        opts.mode(RequestMode::Cors);
        
        let request = Request::new_with_str_and_init(url, &opts)?;
        
        let resp_value = JsFuture::from(window.fetch_with_request(&request)).await?;
        let resp: Response = resp_value.dyn_into()?;
        
        if !resp.ok() {
            return Err(JsValue::from_str(&format!("Failed to fetch {}: {}", url, resp.status())));
        }
        
        let array_buffer = JsFuture::from(resp.array_buffer()?).await?;
        let uint8_array = js_sys::Uint8Array::new(&array_buffer);
        let data = uint8_array.to_vec();
        
        // Determine asset type from URL extension
        let asset = if url.ends_with(".png") || url.ends_with(".jpg") || url.ends_with(".jpeg") || url.ends_with(".gif") || url.ends_with(".webp") {
            Asset::Image(data)
        } else if url.ends_with(".ttf") || url.ends_with(".otf") || url.ends_with(".woff") || url.ends_with(".woff2") {
            Asset::Font(data)
        } else if url.ends_with(".mp3") || url.ends_with(".wav") || url.ends_with(".ogg") {
            Asset::Audio(data)
        } else {
            Asset::Binary(data)
        };
        
        Ok(asset)
    }
    
    pub fn get_cached_asset(&self, url: &str) -> Option<&Asset> {
        self.cache.get(url)
    }
    
    pub fn preload_assets(&mut self, urls: &[&str]) -> Vec<JsFuture> {
        let mut futures = Vec::new();
        
        for url in urls {
            if !self.cache.contains_key(*url) {
                // Start loading but don't await
                let future = self.start_loading(url);
                futures.push(future);
            }
        }
        
        futures
    }
    
    fn start_loading(&self, url: &str) -> JsFuture {
        let window = web_sys::window().unwrap();
        
        let mut opts = RequestInit::new();
        opts.method("GET");
        opts.mode(RequestMode::Cors);
        
        let request = Request::new_with_str_and_init(url, &opts).unwrap();
        JsFuture::from(window.fetch_with_request(&request))
    }
    
    pub fn clear_cache(&mut self) {
        self.cache.clear();
    }
    
    pub fn cache_size(&self) -> usize {
        self.cache.len()
    }
}