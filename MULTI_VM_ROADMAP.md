# Multi-VM Architecture Roadmap

## Phase 1: Foundation & Interface Design
**Goal**: Create VM abstraction without breaking existing functionality

### kryon-runtime
- [ ] **Create VM trait interface**
  ```rust
  pub trait ScriptVM {
      fn initialize() -> Result<Self> where Self: Sized;
      fn load_script(&mut self, name: &str, code: &str) -> Result<()>;
      fn call_function(&mut self, name: &str, args: Vec<ScriptValue>) -> Result<ScriptValue>;
      fn set_global_variable(&mut self, name: &str, value: ScriptValue) -> Result<()>;
      fn get_global_variable(&self, name: &str) -> Option<ScriptValue>;
      fn get_pending_changes(&mut self) -> Result<HashMap<String, String>>;
  }
  ```

- [ ] **Create shared data layer**
  ```rust
  pub struct SharedScriptData {
      variables: HashMap<String, String>,
      template_engine: TemplateEngine,
      change_listeners: Vec<Box<dyn ChangeListener>>,
  }
  ```

- [ ] **Refactor current ScriptSystem**
  - Extract Lua-specific code into `LuaVM` struct
  - Make current system implement the new `ScriptVM` trait
  - Maintain 100% backward compatibility

### kryon-compiler
- [ ] **Validate script languages at compile time**
  ```kry
  @function "lua" increment() { ... }         // Default VM (always available)
  @function "javascript" decrement() { ... }  // Requires javascript-vm feature
  @function "python" process_data() { ... }   // Requires python-vm feature
  ```

- [ ] **Update script validation**
  - Check script language against build-time enabled VMs
  - Error if script uses unavailable VM (e.g., JavaScript without javascript-vm feature)
  - Lua is always available (no feature flag needed)

### kryon-docs
- [ ] **Document multi-VM concept**
  - New section: "Script Languages & VMs"
  - VM selection guide
  - Performance comparison table

---

## Phase 2: VM Registry & Configuration
**Goal**: Pluggable VM system with runtime configuration

### kryon-runtime
- [ ] **Create VM registry**
  ```rust
  pub struct VMRegistry {
      available_vms: HashMap<String, Box<dyn VMFactory>>,
      active_vms: HashMap<String, Box<dyn ScriptVM>>,
  }
  
  pub trait VMFactory {
      fn create_vm() -> Result<Box<dyn ScriptVM>>;
      fn language_name() -> &'static str;
      fn is_available() -> bool; // Check if VM can be loaded
  }
  ```

- [ ] **Update ScriptSystem architecture**
  ```rust
  pub struct ScriptSystem {
      vm_registry: VMRegistry,
      shared_data: SharedScriptData,
      vm_config: VMConfig,
  }
  ```

- [ ] **Create VM configuration**
  ```rust
  pub struct VMConfig {
      enabled_vms: Vec<String>,
      default_vm: String,
      vm_settings: HashMap<String, VMSettings>,
  }
  ```

### kryon-compiler
- [ ] **Add build-time VM validation**
  - Check if script languages match build-time enabled VMs  
  - Error on missing VM features (clear error messages)
  - No changes to KRB format needed (VMs selected at build time)

### kryon-docs
- [ ] **VM configuration guide**
  - How to enable/disable VMs
  - Performance implications
  - Build-time vs runtime selection

---

## Phase 3: JavaScript VM Implementation  
**Goal**: Add lightweight JavaScript VM as optional backend

### kryon-runtime
- [ ] **Create JavaScript VM implementation**
  ```rust
  // New crate: kryon-runtime-javascript (optional dependency)
  // Use QuickJS for microcontroller compatibility instead of V8
  pub struct JavaScriptVM {
      context: quickjs::Context,
      shared_data: Arc<Mutex<SharedScriptData>>,
  }
  
  impl ScriptVM for JavaScriptVM { ... }
  ```

- [ ] **Add feature flags in kryon-runtime Cargo.toml**
  ```toml
  [features]
  default = ["lua-vm"]
  lua-vm = ["mlua"]
  javascript-vm = ["quickjs", "kryon-runtime-javascript"]  
  python-vm = ["pyo3", "kryon-runtime-python"]
  wren-vm = ["wren", "kryon-runtime-wren"]
  all-vms = ["lua-vm", "javascript-vm", "python-vm", "wren-vm"]
  
  # Preset configurations for different platforms
  minimal = ["lua-vm"]                    # Microcontrollers
  embedded = ["lua-vm", "javascript-vm"]  # Embedded systems  
  desktop = ["lua-vm", "javascript-vm", "python-vm"]  # Desktop apps
  ```

- [ ] **Cross-VM variable synchronization**
  ```rust
  impl SharedScriptData {
      fn sync_variable_to_all_vms(&mut self, name: &str, value: &str) {
          for vm in &mut self.active_vms {
              vm.set_global_variable(name, value.into());
          }
      }
  }
  ```

### kryon-compiler
- [ ] **JavaScript syntax validation**
  - Parse JavaScript functions separately
  - Validate syntax using JavaScript parser
  - Convert to KRB with language metadata

### kryon-docs
- [ ] **JavaScript integration guide**
  - Enabling JavaScript VM
  - JavaScript-specific features
  - Performance comparison with Lua
  - Cross-language variable access examples

---

## Phase 4: Build-Time VM Selection
**Goal**: Configure VMs at build time for optimal binary size

### kryon-renderer build system
- [ ] **Cargo feature matrix**
  ```toml
  # Preset configurations
  [features]
  minimal = ["lua-vm"]
  web = ["lua-vm", "javascript-vm"] 
  data = ["lua-vm", "python-vm"]
  full = ["all-vms"]
  ```

- [ ] **Conditional compilation**
  ```rust
  #[cfg(feature = "javascript-vm")]
  registry.register_vm("javascript", JavaScriptVMFactory);
  
  #[cfg(feature = "python-vm")]
  registry.register_vm("python", PythonVMFactory);
  ```

### kryon-compiler
- [ ] **Build-time VM checking**
  - Detect available VMs at compile time
  - Error if KRY file requires unavailable VM
  - Suggest feature flags for missing VMs

### kryon-docs
- [ ] **Build configuration guide**
  - Feature flag combinations
  - Binary size impact table
  - Platform-specific VM availability

---

## Phase 5: MicroPython & Additional VMs
**Goal**: Complete multi-VM ecosystem

### kryon-runtime
- [ ] **MicroPython VM implementation**
  ```rust
  // New crate: kryon-runtime-micropython
  pub struct MicroPythonVM {
      context: micropython_sys::Context,
      shared_data: Arc<Mutex<SharedScriptData>>,
  }
  ```

- [ ] **Wren VM implementation** (if desired)
- [ ] **Performance benchmarking suite**
- [ ] **VM health monitoring**

### kryon-compiler  
- [ ] **Multi-language validation**
- [ ] **Syntax highlighting hints for IDEs**
- [ ] **Cross-language dependency analysis**

### kryon-docs
- [ ] **Complete language reference**
- [ ] **Performance benchmarks**
- [ ] **Best practices per language**
- [ ] **Migration guides**

---

## Phase 6: Advanced Features
**Goal**: Production-ready multi-VM system

### kryon-runtime
- [ ] **Hot-swappable VMs**
- [ ] **VM resource limits**
- [ ] **Cross-VM debugging support**
- [ ] **VM-specific error handling**

### kryon-compiler
- [ ] **Language-specific optimizations**
- [ ] **Cross-language type checking**
- [ ] **VM recommendation engine**

### kryon-docs
- [ ] **Production deployment guide**
- [ ] **Security considerations**
- [ ] **Advanced optimization techniques**

---

## Implementation Priority

**Immediate** (next 2-3 months):
- Phase 1: Foundation & Interface Design
- Phase 2: VM Registry & Configuration

**Medium-term** (3-6 months):
- Phase 3: JavaScript VM Implementation
- Phase 4: Build-Time VM Selection

**Long-term** (6+ months):
- Phase 5: Python & Additional VMs
- Phase 6: Advanced Features

This roadmap ensures backward compatibility while building toward your pluggable VM vision!