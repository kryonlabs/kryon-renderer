# Kryon Renderer Layout System Modernization Roadmap

This roadmap outlines the systematic modernization of the Kryon Renderer layout system architecture, addressing legacy code removal, property system unification, and CSS feature completeness.

## Phase 1: Legacy Code Removal & Foundation (Weeks 1-4)

### Week 1-2: Remove Legacy FlexboxLayoutEngine
- [x] Remove FlexboxLayoutEngine struct definition (`kryon-layout/src/lib.rs:32-34`)
- [x] Remove FlexboxLayoutEngine implementation blocks (`kryon-layout/src/lib.rs:36-592`)
- [x] Remove LayoutEngine trait implementation for FlexboxLayoutEngine (`kryon-layout/src/lib.rs:53-90`)
- [x] Remove all helper methods (`kryon-layout/src/lib.rs:93-592`):
  - [x] `layout_element_with_scale`
  - [x] `layout_children_with_scale`
  - [x] `layout_flex_children_with_scale`
  - [x] `layout_absolute_children_with_scale`
  - [x] Size computation methods
- [x] Remove FlexItem struct (`kryon-layout/src/flexbox.rs:4-12`)
- [x] Update flexbox module exports to keep only LayoutFlags types (`kryon-layout/src/lib.rs:8,12`)
- [x] Keep `apply_legacy_layout_flags()` for KRB backward compatibility
- [x] Update documentation and module references
- [x] Remove unused imports and fix compilation warnings
- [x] Run full test suite to verify no regressions - Core tests passing, layout module builds successfully

### Week 3-4: Property System Foundation
- [x] Create unified PropertyRegistry enum to replace property IDs
- [x] Implement PropertyMetadata for inheritance, types, defaults
- [x] Create PropertyCache system with O(1) array-based property storage
- [x] Implement property caching system with dependency tracking
- [x] Optimize property lookups from O(n) to O(1) array access
- [x] Create property change notification system via cache invalidation
- [x] Add comprehensive test coverage for property registry and cache
- [x] Consolidate 4 property mapping systems:
  - [ ] KRB Parser Mapping (`krb.rs:669-1212`) - Partially done
  - [x] Style Computer Mapping (`style.rs:152-228`) - Completed
  - [x] Style Inheritance Mapping (`style.rs:78-104`) - Completed
  - [ ] Legacy Layout Flags (`flexbox.rs:38-64`) - Needs integration
- [x] Update StyleComputer to use new registry for property application
- [x] Replace hardcoded property matching with PropertyRegistry-based system
- [ ] Performance benchmark: verify 3x improvement in style computation

## Phase 2: Layout System Unification (Weeks 5-10)

### Week 5-6: Complete TaffyLayoutEngine Migration
- [ ] Remove all FlexboxLayoutEngine usage fallbacks from codebase
- [ ] Enhance TaffyLayoutEngine with missing CSS Grid features:
  - [ ] Implement `grid-area` property parsing and application
  - [ ] Add `grid-template-areas` support
  - [ ] Implement `grid-auto-flow` property
  - [ ] Add `grid-column-start/end`, `grid-row-start/end`
  - [ ] Implement named grid lines support
- [ ] Complete advanced flexbox implementation:
  - [ ] Add `flex-wrap` full implementation in layout
  - [ ] Implement `order` property for flex items
  - [ ] Complete `align-self` support
  - [ ] Add `flex` shorthand property parsing
- [ ] Update all layout tests to use TaffyLayoutEngine exclusively
- [ ] Verify cross-backend compatibility (WGPU, Ratatui, Raylib)

### Week 7-8: Box Model Implementation
- [ ] Implement individual side properties:
  - [ ] `padding-top`, `padding-right`, `padding-bottom`, `padding-left`
  - [ ] `margin-top`, `margin-right`, `margin-bottom`, `margin-left`
  - [ ] `border-top-width`, `border-right-width`, etc.
  - [ ] `border-top-color`, `border-right-color`, etc.
- [ ] Add `box-sizing` property support:
  - [ ] Implement `border-box` sizing mode
  - [ ] Implement `content-box` sizing mode (default)
  - [ ] Update layout calculations for different box models
- [ ] Complete border system enhancements:
  - [ ] Individual border radius properties
  - [ ] Advanced border styles (`dashed`, `dotted`, `double`)
  - [ ] Border shorthand property parsing
- [ ] Add `outline` properties support
- [ ] Update constraint system for new box model properties
- [ ] Optimize layout performance with box model changes

### Week 9-10: Positioning System Overhaul
- [ ] Implement `position: fixed` support:
  - [ ] Add fixed positioning in layout engine
  - [ ] Handle viewport-relative positioning
  - [ ] Update rendering backends for fixed elements
- [ ] Implement `position: sticky` support:
  - [ ] Add sticky positioning calculations
  - [ ] Handle scroll-based positioning updates
  - [ ] Implement sticky boundaries
- [ ] Add complete positioning properties:
  - [ ] `right` property implementation
  - [ ] `bottom` property implementation
  - [ ] `inset` shorthand property
- [ ] Complete `z-index` implementation:
  - [ ] Add z-index sorting in all backends
  - [ ] Handle stacking contexts properly
  - [ ] Optimize z-index performance
- [ ] Performance optimization for absolute positioning
- [ ] Add positioning property tests and examples

## Phase 3: Advanced CSS Features (Weeks 11-18)

### Week 11-12: Typography Enhancement
- [ ] Implement advanced text properties:
  - [ ] `line-height` property parsing and application
  - [ ] `letter-spacing` property support
  - [ ] `word-spacing` property support
  - [ ] `text-decoration` properties (`underline`, `overline`, `line-through`)
- [ ] Add text transformation properties:
  - [ ] `text-transform` (`uppercase`, `lowercase`, `capitalize`)
  - [ ] `text-indent` property
  - [ ] `text-overflow` property (`ellipsis`, `clip`)
  - [ ] `white-space` property (`nowrap`, `pre`, `pre-wrap`)
- [ ] Complete font system:
  - [ ] `font-style` (`italic`, `oblique`)
  - [ ] `font-variant` (`small-caps`)
  - [ ] `font-stretch` property
  - [ ] Numeric `font-weight` values (100-900)
- [ ] Optimize text rendering performance across backends
- [ ] Add comprehensive typography examples and tests

### Week 13-14: Visual Effects Foundation
- [ ] Implement `box-shadow` property:
  - [ ] Multiple shadow support
  - [ ] Inset shadow support
  - [ ] Shadow blur and spread radius
  - [ ] Shadow color and offset
- [ ] Add `text-shadow` property support
- [ ] Enhance `opacity` property:
  - [ ] Proper compositing implementation
  - [ ] Opacity inheritance handling
  - [ ] Performance optimization
- [ ] Create filter effects foundation:
  - [ ] `filter` property parsing
  - [ ] `blur()` filter implementation
  - [ ] `brightness()` filter implementation
  - [ ] `contrast()` filter implementation
- [ ] Implement background enhancements:
  - [ ] `background-image` property
  - [ ] Linear gradient support (`linear-gradient()`)
  - [ ] Radial gradient support (`radial-gradient()`)
  - [ ] Multiple background support
- [ ] Add visual effects examples and performance tests

### Week 15-16: Transform System Integration
- [ ] Integrate transforms into unified property system:
  - [ ] Move transforms from separate Vec storage to property system
  - [ ] Remove transform indices from custom properties
  - [ ] Implement property-based transform inheritance
- [ ] Implement missing transform properties:
  - [ ] `transform-origin` property
  - [ ] `perspective` property
  - [ ] `perspective-origin` property
  - [ ] `transform-style: preserve-3d`
  - [ ] `backface-visibility` property
- [ ] Optimize transform system:
  - [ ] Implement transform matrix caching
  - [ ] Optimize transform computation performance
  - [ ] Add transform composition optimization
- [ ] Complete transform inheritance and cascading
- [ ] Update all backends with unified transform application
- [ ] Add comprehensive 3D transform examples

### Week 17-18: Responsive Design Basics
- [ ] Implement `calc()` function:
  - [ ] CSS calc() expression parser
  - [ ] Runtime calc() evaluation
  - [ ] Unit conversion in calc() expressions
  - [ ] Nested calc() support
- [ ] Add media query foundation:
  - [ ] Basic viewport size media queries
  - [ ] Media query condition parsing
  - [ ] Responsive property resolution
- [ ] Complete viewport units implementation:
  - [ ] Ensure vw, vh units work in layout engine
  - [ ] Add vmin, vmax unit support
  - [ ] Implement responsive unit calculations
- [ ] Create responsive property resolution system:
  - [ ] Conditional property application
  - [ ] Media query-based style selection
  - [ ] Responsive layout invalidation
- [ ] Add responsive design examples and tests

## Phase 4: Performance & Modern Features (Weeks 19-26)

### Week 19-20: Performance Optimization
- [ ] Optimize style computation complexity:
  - [ ] Reduce from O(n²) to O(n) complexity
  - [ ] Implement efficient inheritance traversal
  - [ ] Add style computation profiling
- [ ] Implement layout result caching:
  - [ ] Cache computed layout results
  - [ ] Add intelligent cache invalidation
  - [ ] Implement incremental layout updates
- [ ] Reduce memory usage:
  - [ ] Property system memory optimization
  - [ ] Element storage optimization
  - [ ] Layout cache memory management
- [ ] Performance profiling and benchmarking:
  - [ ] Create performance test suite
  - [ ] Identify remaining bottlenecks
  - [ ] Optimize critical performance paths
- [ ] Cross-backend performance verification

### Week 21-22: Advanced Layout Features
- [ ] Implement `aspect-ratio` property:
  - [ ] Aspect ratio constraint parsing
  - [ ] Layout engine aspect ratio support
  - [ ] Aspect ratio with other constraints
- [ ] Add `overflow` properties:
  - [ ] `overflow: hidden` implementation
  - [ ] `overflow: scroll` implementation
  - [ ] `overflow: auto` implementation
  - [ ] Individual axis overflow control
- [ ] Container queries foundation:
  - [ ] Container query parsing
  - [ ] Container-based media queries
  - [ ] Container query resolution
- [ ] CSS Subgrid support:
  - [ ] Subgrid parsing and implementation
  - [ ] Parent grid integration
  - [ ] Subgrid layout calculations
- [ ] Add advanced layout examples and tests

### Week 23-24: Animation Foundation
- [ ] Create CSS transition system:
  - [ ] Transition property parsing
  - [ ] Property interpolation system
  - [ ] Transition timing functions
- [ ] Implement basic transition properties:
  - [ ] `transition-property`
  - [ ] `transition-duration`
  - [ ] `transition-timing-function`
  - [ ] `transition-delay`
  - [ ] `transition` shorthand
- [ ] Add property interpolation system:
  - [ ] Color interpolation
  - [ ] Numeric value interpolation
  - [ ] Transform interpolation
- [ ] Animation timing and easing:
  - [ ] Built-in easing functions
  - [ ] Custom cubic-bezier functions
  - [ ] Animation frame scheduling
- [ ] Create animation examples and performance tests

### Week 25-26: Cross-Backend Compatibility & Testing
- [ ] Verify all features across backends:
  - [ ] WGPU backend feature verification
  - [ ] Ratatui backend feature verification
  - [ ] Raylib backend feature verification
- [ ] Backend-specific optimizations:
  - [ ] WGPU rendering optimizations
  - [ ] Ratatui text-based rendering optimizations
  - [ ] Raylib 2D/3D rendering optimizations
- [ ] Complete test suite coverage:
  - [ ] Unit tests for all new features
  - [ ] Integration tests across backends
  - [ ] Performance regression tests
  - [ ] Visual snapshot test updates
- [ ] Documentation and migration:
  - [ ] Complete API documentation
  - [ ] Create migration guide for breaking changes
  - [ ] Update examples with new features
  - [ ] Performance optimization guide

## Success Metrics & Validation

### Performance Targets
- [ ] **Style Computation**: Achieve 3x improvement (O(n²) → O(n))
- [ ] **Layout Performance**: Achieve 2x improvement through caching
- [ ] **Memory Usage**: Achieve 30% reduction via property consolidation
- [ ] **Code Duplication**: Achieve 80% reduction in property-related code

### Feature Completeness
- [ ] **CSS Grid**: Achieve 90% compatibility with modern web standards
- [ ] **Flexbox**: Achieve 95% compatibility with CSS Flexbox spec
- [ ] **Box Model**: Achieve 100% compatibility with standard box model
- [ ] **Typography**: Achieve 80% compatibility with common text properties

### Quality Assurance
- [ ] **Snapshot Tests**: Maintain 100% pass rate across all backends
- [ ] **Performance Tests**: Ensure no regressions, measurable improvements
- [ ] **Documentation**: Complete API documentation for all new features
- [ ] **Migration Guide**: Clear upgrade path for existing applications

## Architecture Decision Records

### ADR-1: Property System Unification
- **Status**: [ ] Approved
- **Decision**: Replace 4 separate property systems with unified PropertyRegistry
- **Rationale**: Eliminates 200+ lines of duplicate code, improves performance 3x
- **Impact**: Breaking change for internal APIs, full backward compatibility for KRB files

### ADR-2: Legacy Layout Engine Removal
- **Status**: [ ] Approved
- **Decision**: Remove FlexboxLayoutEngine, keep TaffyLayoutEngine only
- **Rationale**: TaffyLayoutEngine provides superior CSS compatibility and performance
- **Impact**: ~500 lines of code removal, simplified architecture

### ADR-3: Transform System Integration
- **Status**: [ ] Approved
- **Decision**: Integrate transforms into main property system instead of separate storage
- **Rationale**: Enables property caching, simplifies inheritance, improves performance
- **Impact**: Changes to transform API, better cross-backend consistency

---

**Total Estimated Timeline**: 26 weeks
**Estimated Code Changes**: ~2000 lines removed, ~3000 lines added
**Expected Performance Improvement**: 2-3x overall rendering performance
**Backward Compatibility**: 100% for KRB files, breaking changes for internal APIs only