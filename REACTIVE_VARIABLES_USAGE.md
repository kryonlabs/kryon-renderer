# Reactive Template Variables Usage Guide

## Overview

The KryonLabs script system now supports reactive template variables, allowing scripts to interact with @variables directly without requiring getter/setter functions.

## Features

### 1. Direct Variable Access
Variables defined in `@variables` blocks are automatically available in script context:

```lua
-- @variables { counter: "0", message: "Hello" }

-- Direct access - no need for getTemplateVariable()
print("Counter:", counter)
print("Message:", message)
```

### 2. Direct Variable Assignment
Variables can be modified directly, with automatic change tracking:

```lua
-- Direct assignment triggers reactive updates
counter = 5
message = "World"

-- Changes are automatically tracked and applied to the UI
```

### 3. Mathematical Operations
Reactive variables support all standard Lua mathematical operations:

```lua
-- @variables { num1: "10", num2: "5" }

-- All operations work transparently
local sum = num1 + num2        -- 15
local diff = num1 - num2       -- 5
local product = num1 * num2    -- 50
local quotient = num1 / num2   -- 2

-- Modifications through operations
counter = counter + 1
num1 = num1 * 2
```

### 4. String Operations
String concatenation and manipulation work seamlessly:

```lua
-- @variables { greeting: "Hello", name: "World" }

-- String concatenation
local message = greeting .. " " .. name  -- "Hello World"

-- String modification
greeting = greeting .. "!"  -- "Hello!"
```

### 5. Comparison Operations
Variables can be compared directly:

```lua
-- @variables { value: "10", threshold: "5" }

if value > threshold then
    print("Value exceeds threshold")
end

if greeting == "Hello" then
    print("Greeting matches")
end
```

### 6. Function Integration
Variables work naturally in function contexts:

```lua
-- @variables { count: "0" }

function increment()
    count = count + 1
    print("Count is now:", count)
end

function reset()
    count = 0
end
```

## Backward Compatibility

The old getter/setter functions still work for existing code:

```lua
-- Legacy approach (still supported)
local value = getTemplateVariable("counter")
setTemplateVariable("counter", value + 1)

-- New reactive approach (recommended)
local value = counter
counter = counter + 1
```

## Implementation Details

### Reactive Proxy System
Each template variable is wrapped in a reactive proxy that:
- Intercepts all operations (assignment, arithmetic, comparison)
- Automatically tracks changes
- Triggers UI updates when variables change
- Maintains type coercion and operations

### Change Detection
The system automatically detects when variables change and queues updates:
- Variable assignments are tracked
- Mathematical operations are monitored
- String operations are captured
- All changes are batched and applied during the next update cycle

### Performance
- Minimal overhead for variable access
- Changes are batched for efficiency
- No polling or manual tracking required
- Automatic cleanup of completed changes

## Usage Patterns

### Counter Example
```lua
-- @variables { counter: "0" }

function increment()
    counter = counter + 1
end

function decrement()
    counter = counter - 1
end

function reset()
    counter = 0
end
```

### Form Validation
```lua
-- @variables { email: "", password: "", isValid: "false" }

function validateForm()
    local emailValid = string.find(email, "@") ~= nil
    local passwordValid = string.len(password) >= 8
    
    isValid = emailValid and passwordValid
    
    if isValid then
        print("Form is valid")
    else
        print("Form validation failed")
    end
end
```

### Dynamic Content
```lua
-- @variables { title: "Welcome", subtitle: "Please log in" }

function updateContent(userLoggedIn)
    if userLoggedIn then
        title = "Dashboard"
        subtitle = "Welcome back!"
    else
        title = "Login"
        subtitle = "Enter your credentials"
    end
end
```

## Migration from Legacy System

### Old Approach
```lua
function onClick()
    local count = getTemplateVariable("counter")
    setTemplateVariable("counter", tostring(tonumber(count) + 1))
end
```

### New Approach
```lua
function onClick()
    counter = counter + 1
end
```

## Error Handling

The system provides clear error messages for common issues:
- Undefined variables: `"Template variable 'name' not found"`
- Type mismatches: Automatic string conversion with warnings
- Script errors: Detailed stack traces with variable context

## Best Practices

1. **Use Direct Assignment**: Prefer `counter = 5` over `setTemplateVariable("counter", 5)`
2. **Leverage Operations**: Use `counter = counter + 1` instead of manual string conversion
3. **Type Awareness**: Remember that all template variables are strings internally
4. **Function Scope**: Variables are globally accessible across all script functions
5. **Change Batching**: Multiple changes in a single script execution are batched efficiently

## Integration with UI Elements

Reactive variables automatically update UI elements that use them:
- Text elements with `{$variable}` syntax
- Property bindings with template expressions
- Conditional visibility based on variable values
- Dynamic styling based on variable states

This creates a seamless reactive programming experience where script changes immediately reflect in the UI without manual intervention.