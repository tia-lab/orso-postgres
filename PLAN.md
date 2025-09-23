# orso-postgres Implementation Plan

## Project Objective

Migrate orso ORM from libSQL/SQLite backend to PostgreSQL while preserving identical public APIs, enabling distributed database architecture for Mathilde's high-performance cryptocurrency analytics platform.

## Success Criteria

### Functional Requirements
- **API Compatibility**: Zero changes to public trait methods, macro attributes, or error types
- **Compression Support**: Maintain cydec integration for Vec<T> compression (5-10x reduction)
- **Multi-Table Support**: Preserve all `_with_table` method variants for flexible schema design
- **Performance**: Match or exceed original orso throughput and latency characteristics
- **Migration System**: Support zero-loss schema migrations with PostgreSQL DDL

### Technical Requirements
- **PostgreSQL 14+**: Utilize native UUID generation and modern PostgreSQL features
- **Connection Pooling**: Implement efficient connection management with deadpool-postgres
- **Async Compatibility**: Maintain existing Tokio-based async patterns
- **Type Safety**: Preserve compile-time guarantees and error handling patterns
- **Testing Coverage**: 95%+ test coverage matching original orso test suite

## Current State Analysis

### Codebase Assessment
```
Total Files: 14 Rust source files
Core Library: orso/ (11 files, ~3,000 lines)
Macro Library: orso-macros/ (1 file, ~1,600 lines)
Estimated Changes: 40% of codebase (database layer + SQL generation)
Preservation: 60% of codebase (traits, types, utilities, filters, pagination)
```

### Dependencies Analysis
```
REMOVE:
- libsql = "0.9.23"          # SQLite/libSQL driver
- rusqlite = "0.30"          # SQLite fallback

PRESERVE:
- tokio = "1.0"              # Async runtime
- serde = "1.0"              # Serialization
- chrono = "0.4"             # DateTime handling
- cydec = { git = "..." }    # Compression codec
- uuid = "1.0"               # UUID generation

ADD:
- tokio-postgres = "0.7"     # PostgreSQL async driver
- postgres-types = "0.2"     # Type system integration
- deadpool-postgres = "0.12" # Connection pooling
```

### API Surface Analysis
```
Trait Methods: 80+ async methods (PRESERVE ALL)
Value Types: 6 enum variants (PRESERVE ALL)
Macro Attributes: 8 column attributes (PRESERVE ALL)
Error Types: 5 error variants (PRESERVE STRUCTURE)
Utility Functions: 20+ helper functions (PRESERVE MOST)
```

## Implementation Strategy

### Architecture Decisions

**Database Connection Pattern**
```
CURRENT: Direct libSQL connection per operation
TARGET: Connection pooling with deadpool-postgres
RATIONALE: Better resource management and PostgreSQL best practices
```

**SQL Parameter Binding**
```
CURRENT: ? placeholders (SQLite style)
TARGET: $1, $2, $3 placeholders (PostgreSQL style)
IMPACT: Update all SQL generation in operations.rs and query.rs
```

**Type Mapping Strategy**
```
CURRENT: orso::Value -> libsql::Value
TARGET: orso::Value -> Box<dyn ToSql + Send + Sync>
RATIONALE: PostgreSQL type system requires trait-based conversion
```

**Compression Integration**
```
CURRENT: cydec -> Vec<u8> -> SQLite BLOB
TARGET: cydec -> Vec<u8> -> PostgreSQL BYTEA
IMPACT: No changes to compression logic, only storage type mapping
```

### File-by-File Migration Plan

#### Phase 1: Core Infrastructure (Files 1-4)

**1. orso/Cargo.toml**
- Status: READY TO MODIFY
- Changes: Replace libsql/rusqlite with tokio-postgres dependencies
- Effort: 1 hour
- Risk: Low

**2. orso/src/lib.rs**
- Status: READY TO MODIFY
- Changes: Update dependency re-exports and feature flags
- Effort: 2 hours
- Risk: Low

**3. orso/src/error.rs**
- Status: MINOR MODIFICATIONS
- Changes: Update Database error variant to wrap PostgreSQL errors
- Effort: 4 hours
- Risk: Medium (error handling compatibility)

**4. orso/src/database.rs**
- Status: COMPLETE REWRITE
- Changes: Replace libSQL with tokio-postgres connection pooling
- Effort: 16 hours
- Risk: High (core infrastructure change)

#### Phase 2: Data Layer (Files 5-7)

**5. orso/src/types.rs**
- Status: NO CHANGES
- Changes: None (Value enum is database-agnostic)
- Effort: 0 hours
- Risk: None

**6. orso/src/traits.rs**
- Status: ADD UTILITY METHODS
- Changes: Add PostgreSQL value conversion helper methods
- Effort: 8 hours
- Risk: Medium (maintain trait compatibility)

**7. orso/src/operations.rs**
- Status: SUBSTANTIAL MODIFICATIONS
- Changes: Update SQL generation and parameter binding
- Effort: 24 hours
- Risk: High (core CRUD operations)

#### Phase 3: Query System (Files 8-10)

**8. orso/src/query.rs**
- Status: MODERATE MODIFICATIONS
- Changes: Update SQL generation for PostgreSQL syntax
- Effort: 12 hours
- Risk: Medium

**9. orso/src/filters.rs**
- Status: MINOR MODIFICATIONS
- Changes: Update operator SQL generation if needed
- Effort: 4 hours
- Risk: Low

**10. orso/src/migrations.rs**
- Status: SUBSTANTIAL MODIFICATIONS
- Changes: PostgreSQL DDL generation and migration logic
- Effort: 16 hours
- Risk: High (schema evolution critical)

#### Phase 4: Utilities (Files 11-13)

**11. orso/src/pagination.rs**
- Status: NO CHANGES
- Changes: None (LIMIT/OFFSET syntax compatible)
- Effort: 0 hours
- Risk: None

**12. orso/src/utils.rs**
- Status: MINOR MODIFICATIONS
- Changes: Update any database-specific utility functions
- Effort: 4 hours
- Risk: Low

**13. orso/src/macros.rs**
- Status: NO CHANGES
- Changes: None (procedural macro re-exports)
- Effort: 0 hours
- Risk: None

#### Phase 5: Macro System (File 14)

**14. orso-macros/src/lib.rs**
- Status: SUBSTANTIAL MODIFICATIONS
- Changes: Update SQL type mapping and schema generation
- Effort: 20 hours
- Risk: High (affects all generated code)

### Total Effort Estimation
```
Phase 1 (Infrastructure): 23 hours
Phase 2 (Data Layer): 32 hours
Phase 3 (Query System): 20 hours
Phase 4 (Utilities): 4 hours
Phase 5 (Macro System): 20 hours

Total Development: 99 hours (~12-15 work days)
Testing & Integration: 40 hours (~5 work days)
Documentation: 8 hours (~1 work day)

Grand Total: 147 hours (~18-20 work days)
```

## Development Phases

### Week 1-2: Foundation (Phase 1 + 2)
**Deliverables:**
- Updated dependencies and module structure
- PostgreSQL connection pooling implementation
- Basic value conversion system
- Core database operations (connect, execute, query)

**Success Metrics:**
- All tests pass with PostgreSQL backend
- Connection pooling functions correctly
- Basic insert/select operations work

**Risks:**
- PostgreSQL type system complexity
- Connection pooling configuration issues
- Async pattern differences

### Week 3-4: Operations (Phase 2 continued + Phase 3)
**Deliverables:**
- Complete CRUD operations migration
- Query builder PostgreSQL compatibility
- Filter system updates
- Parameter binding system

**Success Metrics:**
- All trait methods function correctly
- Complex queries execute successfully
- Performance matches original orso

**Risks:**
- SQL generation complexity
- Parameter binding edge cases
- Query optimization differences

### Week 5-6: Schema System (Phase 4 + 5)
**Deliverables:**
- Migration system for PostgreSQL
- Macro system updates for PostgreSQL SQL
- Schema generation and validation
- DDL operation support

**Success Metrics:**
- Migrations run successfully
- Generated SQL is valid PostgreSQL
- Schema evolution works correctly

**Risks:**
- PostgreSQL DDL syntax differences
- Migration rollback complexity
- Macro debugging challenges

### Week 7-8: Integration & Testing
**Deliverables:**
- Comprehensive test suite
- Performance benchmarking
- Documentation updates
- Production readiness validation

**Success Metrics:**
- 95%+ test coverage achieved
- Performance meets or exceeds targets
- Full API compatibility verified

**Risks:**
- Performance regressions
- Edge case compatibility issues
- Integration test complexity

## Testing Strategy

### Test Categories

**Unit Tests (200+ tests)**
- Value conversion functions
- SQL generation accuracy
- Parameter binding correctness
- Error handling coverage
- Compression integration

**Integration Tests (50+ tests)**
- Full CRUD operations against PostgreSQL
- Migration system functionality
- Connection pooling behavior
- Concurrent operation handling
- Transaction management

**Performance Tests (10+ benchmarks)**
- Insert throughput (target: 10,000+ ops/sec)
- Select performance (target: 50,000+ ops/sec)
- Compression ratio validation (target: 5-10x)
- Connection pool efficiency
- Memory usage profiling

**Compatibility Tests (30+ scenarios)**
- API signature preservation
- Error type compatibility
- Macro attribute functionality
- Multi-table operation support
- Existing code compilation

### Test Infrastructure

**Database Setup**
```sql
-- Development database
CREATE DATABASE orso_postgres_dev;
CREATE USER orso_dev WITH PASSWORD 'dev_password';
GRANT ALL PRIVILEGES ON DATABASE orso_postgres_dev TO orso_dev;

-- Test database
CREATE DATABASE orso_postgres_test;
CREATE USER orso_test WITH PASSWORD 'test_password';
GRANT ALL PRIVILEGES ON DATABASE orso_postgres_test TO orso_test;
```

**CI/CD Integration**
- GitHub Actions with PostgreSQL service container
- Automated test execution on pull requests
- Performance regression detection
- Code coverage reporting

### Test Data Strategy
- Synthetic data generation for performance tests
- Compressed array test data (Vec<i64> with 1000+ elements)
- Edge case data (NULL values, Unicode strings, large blobs)
- Schema evolution test cases (add/remove columns, index changes)

## Risk Management

### High-Risk Areas

**PostgreSQL Type System Complexity**
- Risk Level: HIGH
- Impact: Core functionality broken
- Mitigation: Extensive type conversion testing, PostgreSQL documentation review
- Contingency: Fallback to simpler type mapping strategy

**Connection Pool Configuration**
- Risk Level: MEDIUM
- Impact: Performance degradation or connection exhaustion
- Mitigation: Load testing, production-like configuration testing
- Contingency: Implement custom connection management if needed

**Macro System SQL Generation**
- Risk Level: HIGH
- Impact: Generated SQL invalid or inefficient
- Mitigation: Comprehensive SQL validation testing, PostgreSQL syntax verification
- Contingency: Manual SQL review and correction process

**Performance Regression**
- Risk Level: MEDIUM
- Impact: Slower than original orso performance
- Mitigation: Continuous benchmarking, PostgreSQL query optimization
- Contingency: Performance tuning sprint, connection pool optimization

### Risk Mitigation Strategies

**Development Approach**
- Test-driven development for critical components
- Incremental migration with rollback capability
- Dual-write period for data consistency validation
- Performance monitoring throughout development

**Quality Assurance**
- Code review requirement for all changes
- Automated testing on multiple PostgreSQL versions
- Memory leak detection and profiling
- Security vulnerability scanning

## Success Metrics

### Functional Metrics
- **API Compatibility**: 100% of existing trait methods preserved
- **Test Coverage**: 95%+ line coverage maintained
- **Feature Parity**: All orso features working with PostgreSQL
- **Migration Success**: Zero data loss during schema evolution

### Performance Metrics
- **Insert Throughput**: e10,000 records/second (batch operations)
- **Query Performance**: e50,000 queries/second (with connection pooling)
- **Latency**: <5ms single record operations, <50ms complex queries
- **Compression Ratio**: 5-10x for Vec<i64> data maintained
- **Memory Usage**: d110% of original orso memory footprint

### Quality Metrics
- **Bug Count**: <5 critical bugs in production
- **Documentation**: 100% of public API documented
- **Code Quality**: Maintainability index e80
- **Security**: No SQL injection vulnerabilities, TLS required

## Deployment Strategy

### Integration Path
1. **Parallel Development**: orso-postgres developed alongside existing orso
2. **Dual-Write Testing**: Write operations to both databases for validation
3. **Gradual Migration**: Read operations switched to orso-postgres incrementally
4. **Write Cutover**: All write operations moved to orso-postgres after validation
5. **Legacy Cleanup**: Remove orso dependency after full migration

### Rollback Plan
- Maintain orso dependency during transition period
- Implement feature flag for database backend selection
- Real-time data synchronization between backends
- Automated rollback triggers on performance degradation
- Manual rollback procedure documented and tested

## Conclusion

This implementation plan provides a systematic approach to migrating orso from SQLite/libSQL to PostgreSQL while maintaining complete API compatibility. The 18-20 work day timeline accounts for the complexity of database backend migration while ensuring thorough testing and quality assurance.

The success of this migration enables Mathilde to leverage PostgreSQL's distributed capabilities, multi-disk storage utilization, and superior analytical query performance while maintaining the familiar orso ORM interface that developers already understand.