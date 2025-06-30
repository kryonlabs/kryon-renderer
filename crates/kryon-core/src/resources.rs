// crates/kryon-core/src/resources.rs
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct ResourceManager {
    pub resources: HashMap<String, ResourceEntry>,
}

#[derive(Debug, Clone)]
pub struct ResourceEntry {
    pub id: String,
    pub resource_type: ResourceType,
    pub path: String,
    pub data: Option<Vec<u8>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResourceType {
    Image,
    Font,
    Audio,
    Video,
    Script,
    Style,
    Data,
}

impl Default for ResourceManager {
    fn default() -> Self {
        Self {
            resources: HashMap::new(),
        }
    }
}

impl ResourceManager {
    pub fn new() -> Self {
        Self::default()
    }
    
    pub fn add_resource(&mut self, entry: ResourceEntry) {
        self.resources.insert(entry.id.clone(), entry);
    }
    
    pub fn get_resource(&self, id: &str) -> Option<&ResourceEntry> {
        self.resources.get(id)
    }
}