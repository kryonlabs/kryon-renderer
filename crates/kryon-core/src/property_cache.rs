// crates/kryon-core/src/property_cache.rs

use std::collections::HashMap;
use std::cell::RefCell;
use crate::{PropertyValue, PropertyId, ElementId, InteractionState};

/// Efficient property caching system to optimize property lookups
/// Replaces the multiple O(n) lookups with O(1) cached access
pub struct PropertyCache {
    /// Cache for computed property values per element and interaction state
    cache: RefCell<HashMap<(ElementId, InteractionState, PropertyId), PropertyValue>>,
    /// Cache for computed styles per element and interaction state
    style_cache: RefCell<HashMap<(ElementId, InteractionState), ComputedPropertySet>>,
    /// Dependency tracking for cache invalidation
    dependencies: RefCell<HashMap<ElementId, Vec<ElementId>>>,
}

/// Represents a complete set of computed properties for an element
#[derive(Debug, Clone)]
pub struct ComputedPropertySet {
    /// Array-based storage for fast O(1) property access
    /// Index corresponds to PropertyId::as_u8()
    properties: [Option<PropertyValue>; 256],
    /// Timestamp for cache invalidation
    last_modified: u64,
}

impl ComputedPropertySet {
    pub fn new() -> Self {
        Self {
            properties: [const { None }; 256],
            last_modified: 0,
        }
    }
    
    pub fn get(&self, property_id: PropertyId) -> Option<&PropertyValue> {
        let index = property_id.as_u8() as usize;
        self.properties[index].as_ref()
    }
    
    pub fn set(&mut self, property_id: PropertyId, value: PropertyValue) {
        let index = property_id.as_u8() as usize;
        self.properties[index] = Some(value);
        self.last_modified = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64;
    }
    
    pub fn remove(&mut self, property_id: PropertyId) {
        let index = property_id.as_u8() as usize;
        self.properties[index] = None;
        self.last_modified = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64;
    }
    
    pub fn last_modified(&self) -> u64 {
        self.last_modified
    }
}

impl Default for ComputedPropertySet {
    fn default() -> Self {
        Self::new()
    }
}

impl PropertyCache {
    pub fn new() -> Self {
        Self {
            cache: RefCell::new(HashMap::new()),
            style_cache: RefCell::new(HashMap::new()),
            dependencies: RefCell::new(HashMap::new()),
        }
    }
    
    /// Get a cached property value for an element in a specific state
    pub fn get_property(
        &self,
        element_id: ElementId,
        state: InteractionState,
        property_id: PropertyId,
    ) -> Option<PropertyValue> {
        let cache = self.cache.borrow();
        let key = (element_id, state, property_id);
        cache.get(&key).cloned()
    }
    
    /// Set a cached property value for an element in a specific state
    pub fn set_property(
        &self,
        element_id: ElementId,
        state: InteractionState,
        property_id: PropertyId,
        value: PropertyValue,
    ) {
        let mut cache = self.cache.borrow_mut();
        let key = (element_id, state, property_id);
        cache.insert(key, value);
    }
    
    /// Get a cached computed property set for an element
    pub fn get_computed_properties(
        &self,
        element_id: ElementId,
        state: InteractionState,
    ) -> Option<ComputedPropertySet> {
        let cache = self.style_cache.borrow();
        let key = (element_id, state);
        cache.get(&key).cloned()
    }
    
    /// Set a cached computed property set for an element
    pub fn set_computed_properties(
        &self,
        element_id: ElementId,
        state: InteractionState,
        properties: ComputedPropertySet,
    ) {
        let mut cache = self.style_cache.borrow_mut();
        let key = (element_id, state);
        cache.insert(key, properties);
    }
    
    /// Invalidate cache for a specific element
    pub fn invalidate_element(&self, element_id: ElementId) {
        // Remove all cached entries for this element
        let mut cache = self.cache.borrow_mut();
        cache.retain(|(id, _, _), _| *id != element_id);
        
        let mut style_cache = self.style_cache.borrow_mut();
        style_cache.retain(|(id, _), _| *id != element_id);
        
        // Also invalidate dependent elements
        let deps = self.dependencies.borrow();
        if let Some(dependents) = deps.get(&element_id) {
            for &dependent_id in dependents {
                cache.retain(|(id, _, _), _| *id != dependent_id);
                style_cache.retain(|(id, _), _| *id != dependent_id);
            }
        }
    }
    
    /// Invalidate cache for a specific property across all elements
    pub fn invalidate_property(&self, property_id: PropertyId) {
        let mut cache = self.cache.borrow_mut();
        cache.retain(|(_, _, prop_id), _| *prop_id != property_id);
    }
    
    /// Add a dependency relationship between elements
    pub fn add_dependency(&self, parent_id: ElementId, child_id: ElementId) {
        let mut deps = self.dependencies.borrow_mut();
        deps.entry(parent_id)
            .or_insert_with(Vec::new)
            .push(child_id);
    }
    
    /// Remove a dependency relationship
    pub fn remove_dependency(&self, parent_id: ElementId, child_id: ElementId) {
        let mut deps = self.dependencies.borrow_mut();
        if let Some(dependents) = deps.get_mut(&parent_id) {
            dependents.retain(|&id| id != child_id);
        }
    }
    
    /// Clear all caches
    pub fn clear(&self) {
        self.cache.borrow_mut().clear();
        self.style_cache.borrow_mut().clear();
        self.dependencies.borrow_mut().clear();
    }
    
    /// Get cache statistics for debugging
    pub fn stats(&self) -> PropertyCacheStats {
        let cache = self.cache.borrow();
        let style_cache = self.style_cache.borrow();
        let deps = self.dependencies.borrow();
        
        PropertyCacheStats {
            property_cache_size: cache.len(),
            style_cache_size: style_cache.len(),
            dependency_count: deps.len(),
            total_dependencies: deps.values().map(|v| v.len()).sum(),
        }
    }
}

impl Default for PropertyCache {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct PropertyCacheStats {
    pub property_cache_size: usize,
    pub style_cache_size: usize,
    pub dependency_count: usize,
    pub total_dependencies: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::PropertyValue;
    use glam::Vec4;
    
    #[test]
    fn test_property_cache_basic() {
        let cache = PropertyCache::new();
        let element_id = 1;
        let state = InteractionState::Normal;
        let property_id = PropertyId::BackgroundColor;
        let value = PropertyValue::Color(Vec4::new(1.0, 0.0, 0.0, 1.0));
        
        // Test cache miss
        assert!(cache.get_property(element_id, state, property_id).is_none());
        
        // Test cache set and hit
        cache.set_property(element_id, state, property_id, value.clone());
        let cached_value = cache.get_property(element_id, state, property_id);
        assert!(cached_value.is_some());
        
        // Test cache invalidation
        cache.invalidate_element(element_id);
        assert!(cache.get_property(element_id, state, property_id).is_none());
    }
    
    #[test]
    fn test_computed_property_set() {
        let mut prop_set = ComputedPropertySet::new();
        let property_id = PropertyId::BackgroundColor;
        let value = PropertyValue::Color(Vec4::new(1.0, 0.0, 0.0, 1.0));
        
        // Test empty set
        assert!(prop_set.get(property_id).is_none());
        
        // Test set and get
        prop_set.set(property_id, value.clone());
        let retrieved = prop_set.get(property_id);
        assert!(retrieved.is_some());
        
        // Test remove
        prop_set.remove(property_id);
        assert!(prop_set.get(property_id).is_none());
    }
    
    #[test]
    fn test_dependency_tracking() {
        let cache = PropertyCache::new();
        let parent_id = 1;
        let child_id = 2;
        
        // Add dependency
        cache.add_dependency(parent_id, child_id);
        
        // Cache some values for both elements
        let prop_id = PropertyId::TextColor;
        let value = PropertyValue::Color(Vec4::new(0.0, 1.0, 0.0, 1.0));
        cache.set_property(parent_id, InteractionState::Normal, prop_id, value.clone());
        cache.set_property(child_id, InteractionState::Normal, prop_id, value.clone());
        
        // Verify both are cached
        assert!(cache.get_property(parent_id, InteractionState::Normal, prop_id).is_some());
        assert!(cache.get_property(child_id, InteractionState::Normal, prop_id).is_some());
        
        // Invalidate parent should invalidate child too
        cache.invalidate_element(parent_id);
        assert!(cache.get_property(parent_id, InteractionState::Normal, prop_id).is_none());
        assert!(cache.get_property(child_id, InteractionState::Normal, prop_id).is_none());
    }
    
    #[test]
    fn test_cache_stats() {
        let cache = PropertyCache::new();
        let stats = cache.stats();
        assert_eq!(stats.property_cache_size, 0);
        assert_eq!(stats.style_cache_size, 0);
        assert_eq!(stats.dependency_count, 0);
        
        // Add some data
        cache.set_property(1, InteractionState::Normal, PropertyId::BackgroundColor, 
                          PropertyValue::Color(Vec4::ONE));
        cache.add_dependency(1, 2);
        
        let stats = cache.stats();
        assert_eq!(stats.property_cache_size, 1);
        assert_eq!(stats.dependency_count, 1);
        assert_eq!(stats.total_dependencies, 1);
    }
}