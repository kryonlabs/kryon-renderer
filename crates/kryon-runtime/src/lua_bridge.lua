-- =============================================================================
--  Kryon-Lua Bridge: Core API for Scripting
-- =============================================================================
-- This script sets up the API that allows Lua code to interact with the
-- Kryon UI engine. It provides a DOM-like interface for manipulating
-- elements, handling events, and managing state.
--

-- =============================================================================
--  1. Global State & Pending Changes
-- =============================================================================
-- These tables store changes made by scripts during a single frame.
-- They are collected by the Rust runtime after each event handler runs to
-- update the UI state efficiently.

_pending_style_changes      = {} -- { [element_id] = new_style_id }
_pending_state_changes      = {} -- { [element_id] = is_checked (boolean) }
_pending_text_changes       = {} -- { [element_id] = new_text (string) }
_pending_visibility_changes = {} -- { [element_id] = is_visible (boolean) }

-- Event listener system state
_event_listeners            = {} -- { [event_type] = {callback1, callback2, ...} }
_ready_callbacks            = {} -- Callbacks to run when the UI is fully loaded
_is_ready                   = false


-- =============================================================================
--  2. Core DOM-like API
-- =============================================================================

---
-- Retrieves an element by its string ID and returns a proxy object
-- that can be used to manipulate it.
---@param element_id string The string ID of the element to retrieve.
---@return table|nil A proxy object for the element, or nil if not found.
--
function getElementById(element_id)
    -- _element_ids is a table provided by the Rust runtime mapping string IDs to numeric IDs.
    local numeric_id = _element_ids[element_id]

    if numeric_id then
        -- Return a proxy table with methods to manipulate the element.
        -- This avoids exposing the raw element data directly to the script.
        return {
            id = element_id,
            numeric_id = numeric_id,

            -- Queues a style change for the element.
            setStyle = function(self, style_name)
                local style_id = _style_ids[style_name]
                if style_id then
                    _pending_style_changes[self.numeric_id] = style_id
                    print("Queuing style change for element '" .. self.id .. "' to style '" .. style_name .. "' (ID: " .. style_id .. ")")
                else
                    print("Error: Style '" .. style_name .. "' not found.")
                end
            end,

            -- Queues a checked state change (for checkboxes, radio buttons, etc.).
            setChecked = function(self, checked)
                _pending_state_changes[self.numeric_id] = checked
                print("Queuing checked state change for element '" .. self.id .. "' to " .. tostring(checked))
            end,

            -- Queues a text content change.
            setText = function(self, text)
                _pending_text_changes[self.numeric_id] = tostring(text)
                print("Queuing text change for element '" .. self.id .. "' to: " .. tostring(text))
            end,

            -- Gets the current text of the element, considering pending changes first.
            getText = function(self)
                if _pending_text_changes[self.numeric_id] ~= nil then
                    return _pending_text_changes[self.numeric_id]
                end
                -- _elements_data is provided by the Rust runtime.
                if _elements_data and _elements_data[self.numeric_id] then
                    return _elements_data[self.numeric_id].text or ""
                end
                return ""
            end,

            -- Queues a visibility change.
            setVisible = function(self, visible)
                _pending_visibility_changes[self.numeric_id] = visible
                print("Queuing visibility change for element '" .. self.id .. "' to " .. tostring(visible))
            end,

            -- Gets the current visibility of the element, considering pending changes.
            getVisible = function(self)
                if _pending_visibility_changes[self.numeric_id] ~= nil then
                    return _pending_visibility_changes[self.numeric_id]
                end
                if _elements_data and _elements_data[self.numeric_id] then
                    return _elements_data[self.numeric_id].visible
                end
                return true -- Default to visible if not found
            end,

            -- DOM Traversal Methods
            getParent = function(self) return _get_parent_element(self.numeric_id) end,
            getChildren = function(self) return _get_children_elements(self.numeric_id) end,
            getFirstChild = function(self)
                local children = _get_children_elements(self.numeric_id)
                return children[1] or nil
            end,
            getLastChild = function(self)
                local children = _get_children_elements(self.numeric_id)
                return children[#children] or nil
            end,
            getNextSibling = function(self) return _get_next_sibling(self.numeric_id) end,
            getPreviousSibling = function(self) return _get_previous_sibling(self.numeric_id) end
        }
    else
        print("Error: Element with ID '" .. tostring(element_id) .. "' not found.")
        return nil
    end
end

-- Alias for backward compatibility.
get_element = getElementById

---
-- Retrieves a table of custom properties for a given component/element.
---@param element_id string|number The ID of the element.
---@return table A table of properties, or an empty table if none.
--
function getComponentProperties(element_id)
    local props = _component_properties[element_id]
    if not props and type(element_id) == "string" then
        local numeric_id = _element_ids[element_id]
        if numeric_id then
            props = _component_properties[numeric_id]
        end
    end
    return props or {}
end

---
-- Retrieves a single custom property value for a given component/element.
---@param element_id string|number The ID of the element.
---@param property_name string The name of the property to retrieve.
---@return any The property value, or nil if not found.
--
function getComponentProperty(element_id, property_name)
    local props = getComponentProperties(element_id)
    return props[property_name]
end


-- =============================================================================
--  3. DOM Traversal & Querying (Internal Helpers)
-- =============================================================================
-- These internal functions are called by the public API and query the
-- _elements_data table provided by the Rust runtime.

---
-- Internal helper to create a Lua-side proxy for a raw element.
---@param element_id number The numeric ID of the element.
---@return table|nil The proxy object.
--
function _create_element_proxy(element_id)
    local element_data = _elements_data[element_id]
    if not element_data then return nil end
    
    -- This proxy object is read-only and used for traversal.
    -- To modify an element, one must first get it via `getElementById`.
    return {
        id = element_data.id,
        numeric_id = element_id,
        element_type = element_data.element_type,
        visible = element_data.visible,
        text = element_data.text
    }
end

function _get_parent_element(element_id)
    local element_data = _elements_data[element_id]
    if element_data and element_data.parent_id then
        return _create_element_proxy(element_data.parent_id)
    end
    return nil
end

function _get_children_elements(element_id)
    local result = {}
    local element_data = _elements_data[element_id]
    if element_data and element_data.children then
        for _, child_id in ipairs(element_data.children) do
            local child_proxy = _create_element_proxy(child_id)
            if child_proxy then table.insert(result, child_proxy) end
        end
    end
    return result
end

function _get_next_sibling(element_id)
    local element_data = _elements_data[element_id]
    if element_data and element_data.parent_id then
        local parent_data = _elements_data[element_data.parent_id]
        if parent_data and parent_data.children then
            for i, child_id in ipairs(parent_data.children) do
                if child_id == element_id and i < #parent_data.children then
                    return _create_element_proxy(parent_data.children[i + 1])
                end
            end
        end
    end
    return nil
end

function _get_previous_sibling(element_id)
    local element_data = _elements_data[element_id]
    if element_data and element_data.parent_id then
        local parent_data = _elements_data[element_data.parent_id]
        if parent_data and parent_data.children then
            for i, child_id in ipairs(parent_data.children) do
                if child_id == element_id and i > 1 then
                    return _create_element_proxy(parent_data.children[i - 1])
                end
            end
        end
    end
    return nil
end

function _get_elements_by_tag(tag_name)
    local result = {}
    for element_id, element_data in pairs(_elements_data) do
        if element_data.element_type:lower() == tag_name:lower() then
            local proxy = _create_element_proxy(element_id)
            if proxy then table.insert(result, proxy) end
        end
    end
    return result
end

function _get_elements_by_class(class_name)
    local result = {}
    for element_id, element_data in pairs(_elements_data) do
        local style_data = _styles_data[element_data.style_id]
        if style_data and style_data.name == class_name then
            local proxy = _create_element_proxy(element_id)
            if proxy then table.insert(result, proxy) end
        end
    end
    return result
end

function _query_selector_all(selector)
    local result = {}
    if selector:sub(1, 1) == '#' then
        local id = selector:sub(2)
        local element = getElementById(id)
        if element then table.insert(result, element) end
    elseif selector:sub(1, 1) == '.' then
        result = _get_elements_by_class(selector:sub(2))
    else
        result = _get_elements_by_tag(selector)
    end
    return result
end

function querySelector(selector)
    local results = _query_selector_all(selector)
    return results[1] or nil
end

function getRootElement()
    return _get_root_element()
end


-- =============================================================================
--  4. Event System
-- =============================================================================

---
-- Registers a callback to be executed when the UI is fully loaded and ready,
-- similar to DOMContentLoaded in web development.
---@param callback function The function to execute.
--
function onReady(callback)
    if type(callback) ~= "function" then
        print("Error: onReady(callback) - callback must be a function.")
        return
    end

    if _is_ready then
        -- If UI is already ready, execute immediately.
        local success, error = pcall(callback)
        if not success then
            print("Error in immediate onReady callback: " .. tostring(error))
        end
    else
        -- Otherwise, queue for later execution.
        table.insert(_ready_callbacks, callback)
    end
end

-- Internal function called by the Rust runtime to signal readiness.
function _mark_ready()
    _is_ready = true
end

---
-- Adds a global event listener for document-level events like 'keydown'.
---@param event_type string The type of event to listen for (e.g., "keydown").
---@param callback function The function to call when the event occurs.
--
function addEventListener(event_type, callback)
    if type(event_type) ~= "string" or type(callback) ~= "function" then
        print("Error: addEventListener(event_type, callback) requires a string and a function.")
        return
    end

    if not _event_listeners[event_type] then
        _event_listeners[event_type] = {}
    end

    table.insert(_event_listeners[event_type], callback)
    print("Added event listener for '" .. event_type .. "'.")
end

-- Internal function called by the Rust runtime to trigger an event.
function _trigger_event(event_type, event_data)
    if _event_listeners[event_type] then
        for _, callback in ipairs(_event_listeners[event_type]) do
            pcall(callback, event_data or {})
        end
    end
end

-- Convenience wrappers for common events
function onKeyDown(callback) addEventListener('keydown', callback) end
function onKeyUp(callback) addEventListener('keyup', callback) end
function onResize(callback) addEventListener('resize', callback) end
function onLoad(callback) addEventListener('load', callback) end


-- =============================================================================
--  5. Internal Getter Functions for the Rust Runtime
-- =============================================================================
-- These functions are called by the Rust runtime to retrieve the state
-- changes that scripts have queued up.

---
-- Helper function to clear a table in-place. This is crucial because
-- re-assigning a global variable (e.g., `_my_table = {}`) can trigger
-- the __newindex metamethod, which can cause issues. Modifying the
-- table's contents directly avoids this.
---@param t table The table to clear.
--
function _clear_table_in_place(t)
    for k in pairs(t) do
        t[k] = nil
    end
end

---
-- Creates a copy of a table.
---@param t table The table to copy.
---@return table A new table with the same key-value pairs.
--
function _copy_table(t)
    local copy = {}
    for k, v in pairs(t) do
        copy[k] = v
    end
    return copy
end

-- The following functions get changes without clearing them.
-- Changes are cleared separately after both DOM and template variable changes are applied.

function _get_pending_style_changes()
    return _copy_table(_pending_style_changes)
end

function _get_pending_state_changes()
    return _copy_table(_pending_state_changes)
end

function _get_pending_text_changes()
    return _copy_table(_pending_text_changes)
end

function _get_pending_visibility_changes()
    return _copy_table(_pending_visibility_changes)
end

-- This getter is part of the reactive variable system.
-- Get template variable changes without clearing them.
function _get_reactive_template_variable_changes()
    return _copy_table(_template_variable_changes)
end

-- Clear all DOM changes without returning them
function _clear_dom_changes()
    _clear_table_in_place(_pending_style_changes)
    _clear_table_in_place(_pending_state_changes)
    _clear_table_in_place(_pending_text_changes)
    _clear_table_in_place(_pending_visibility_changes)
end

-- Clear template variable changes without returning them
function _clear_template_variable_changes()
    _clear_table_in_place(_template_variable_changes)
end
