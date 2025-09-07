# Comprehensive Testing Suite

This directory contains a comprehensive testing suite for the Polygon POL Indexer that validates all aspects of the system from individual components to end-to-end workflows.

## Test Categories

### 1. Unit Tests (`src/` modules)

- **Location**: Embedded in source files and `src/database/tests.rs`
- **Purpose**: Test individual functions and components in isolation
- **Coverage**: Database operations, data models, address classification, transfer detection
- **Run with**: `cargo test --lib`

### 2. Integration Tests

#### Existing Integration Tests

- **Files**: `integration_block_monitoring.rs`, `integration_block_processing.rs`
- **Purpose**: Test component interactions and workflows
- **Coverage**: Block monitoring, RPC client integration, database integration
- **Run with**: `cargo test --test integration_block_monitoring --test integration_block_processing`

#### Real Polygon Testnet Integration (`integration_polygon_testnet.rs`)

- **Purpose**: Test against real Polygon Mumbai testnet
- **Coverage**: Real RPC connectivity, actual block fetching, live POL transfer detection
- **Requirements**: Network connectivity
- **Run with**: `cargo test --test integration_polygon_testnet -- --ignored`

### 3. End-to-End Workflow Tests (`end_to_end_workflow.rs`)

- **Purpose**: Test complete system workflows from start to finish
- **Coverage**:
  - Complete block processing pipeline
  - Database persistence across restarts
  - CLI query functionality
  - Error recovery and resilience
  - Data consistency validation
  - Concurrent access patterns
- **Run with**: `cargo test --test end_to_end_workflow`

### 4. Performance Tests (`performance_tests.rs`)

- **Purpose**: Validate system performance under load
- **Coverage**:
  - Bulk database insert performance (target: >50 transfers/second)
  - Query performance (target: >1000 queries/second)
  - Concurrent database access
  - Memory usage patterns
  - Realistic block processing simulation
- **Run with**: `cargo test --test performance_tests --release`

### 5. Network Failure and Recovery Tests (`network_failure_recovery.rs`)

- **Purpose**: Test system resilience under network conditions
- **Coverage**:
  - RPC endpoint failures
  - Network timeouts and retries
  - Graceful degradation
  - Recovery after extended outages
  - Database consistency during network issues
- **Dependencies**: `wiremock` for mock HTTP servers
- **Run with**: `cargo test --test network_failure_recovery`

### 6. Validation Tests (`validation_tests.rs`)

- **Purpose**: Validate against known data patterns and edge cases
- **Coverage**:
  - Known POL transfer transaction patterns
  - Event log parsing accuracy
  - Binance address classification
  - Amount parsing and precision
  - Edge cases and error conditions
  - Complex net flow calculations
- **Run with**: `cargo test --test validation_tests`

### 7. Performance Benchmarks (`benches/database_benchmarks.rs`)

- **Purpose**: Detailed performance profiling using Criterion
- **Coverage**:
  - Database insert benchmarks
  - Query performance benchmarks
  - Net flow calculation benchmarks
- **Run with**: `cargo bench`
- **Output**: HTML reports in `target/criterion/`

## Test Requirements

### Dependencies

The comprehensive test suite requires additional dependencies:

```toml
[dev-dependencies]
tokio-test = "0.4"      # Async test utilities
mockito = "1.2"         # HTTP mocking (legacy tests)
criterion = "0.5"       # Performance benchmarking
tempfile = "3.8"        # Temporary file management
serial_test = "3.0"     # Sequential test execution
wiremock = "0.5"        # HTTP mocking (new tests)
```

### System Requirements

- **Memory**: 1GB RAM minimum for performance tests
- **Storage**: 100MB for temporary test databases
- **Network**: Internet connectivity for testnet integration tests
- **Time**: Full suite takes 5-15 minutes depending on system

## Running Tests

### Quick Test Run

```bash
# Run all tests except network-dependent ones
cargo test
```

### Comprehensive Test Suite

```bash
# Linux/macOS
./scripts/run_comprehensive_tests.sh

# Windows
scripts\run_comprehensive_tests.bat
```

### Individual Test Categories

```bash
# Unit tests only
cargo test --lib

# Integration tests
cargo test --test integration_block_monitoring
cargo test --test integration_block_processing

# End-to-end tests
cargo test --test end_to_end_workflow

# Performance tests (release mode for accurate timing)
cargo test --test performance_tests --release

# Network failure tests
cargo test --test network_failure_recovery

# Validation tests
cargo test --test validation_tests

# Real testnet tests (requires network)
cargo test --test integration_polygon_testnet -- --ignored

# Performance benchmarks
cargo bench
```

## Test Data and Patterns

### Mock Data

Tests use realistic mock data that matches real-world patterns:

- Valid Ethereum addresses (42 characters, 0x prefix)
- Realistic block numbers (40M+ for Polygon)
- Proper POL token amounts (18 decimal places)
- Known Binance wallet addresses
- Valid transaction hashes (64 hex characters)

### Known Test Patterns

- **Large transfers**: 10+ POL to test precision
- **Small transfers**: Wei-level amounts to test edge cases
- **Binance addresses**: All 6 known Binance addresses tested
- **Transfer directions**: Inflows, outflows, and non-relevant transfers
- **Error conditions**: Invalid data, network failures, database errors

## Performance Targets

### Database Operations

- **Insert Performance**: >50 transfers/second
- **Query Performance**: >1000 queries/second
- **Bulk Operations**: >30 transfers/second concurrent
- **Memory Usage**: <100MB for 1000 transfers

### Network Operations

- **RPC Calls**: <5 second timeout with retries
- **Block Processing**: >10 blocks/second
- **Error Recovery**: <60 seconds for network recovery

### System Metrics

- **Startup Time**: <5 seconds
- **Query Response**: <1 second for simple queries
- **Data Consistency**: 100% accuracy across restarts

## Validation Criteria

### Data Accuracy

- ✅ All POL transfers correctly identified
- ✅ Binance addresses properly classified
- ✅ Transfer amounts parsed with full precision
- ✅ Net flow calculations mathematically correct
- ✅ Database consistency maintained across operations

### System Reliability

- ✅ Graceful handling of network failures
- ✅ Automatic retry with exponential backoff
- ✅ Data persistence across system restarts
- ✅ Concurrent access without data corruption
- ✅ Memory usage remains stable under load

### Performance Standards

- ✅ Real-time processing capability (2-second blocks)
- ✅ Scalable to handle high transaction volumes
- ✅ Efficient database operations
- ✅ Responsive query interface
- ✅ Minimal resource consumption

## Test Results Interpretation

### Success Criteria

All tests must pass for the system to be considered ready for deployment:

1. **Unit Tests**: 100% pass rate
2. **Integration Tests**: 100% pass rate
3. **End-to-End Tests**: 100% pass rate
4. **Performance Tests**: Meet all performance targets
5. **Network Tests**: Handle failures gracefully
6. **Validation Tests**: 100% data accuracy

### Common Issues and Solutions

#### Network-Dependent Test Failures

- **Cause**: Internet connectivity issues or RPC endpoint problems
- **Solution**: Tests are designed to be resilient; temporary network issues should not fail the core functionality tests

#### Performance Test Variations

- **Cause**: System load, hardware differences, or background processes
- **Solution**: Performance targets have reasonable margins; consistent patterns matter more than absolute numbers

#### Database Lock Issues

- **Cause**: Concurrent test execution on the same database
- **Solution**: Tests use temporary databases and proper cleanup

## Continuous Integration

The test suite is designed for CI/CD integration:

```yaml
# Example GitHub Actions workflow
- name: Run Comprehensive Tests
  run: |
    cargo test --lib
    cargo test --test end_to_end_workflow
    cargo test --test performance_tests --release
    cargo test --test network_failure_recovery
    cargo test --test validation_tests
```

## Contributing to Tests

When adding new functionality:

1. **Add unit tests** for individual functions
2. **Update integration tests** for component interactions
3. **Add end-to-end scenarios** for new workflows
4. **Include performance tests** for performance-critical code
5. **Add validation tests** for new data patterns
6. **Update this documentation** with new test descriptions

## Test Maintenance

### Regular Tasks

- Update mock data to reflect real-world changes
- Adjust performance targets based on hardware improvements
- Add new test scenarios for edge cases discovered in production
- Keep dependencies updated for security and compatibility

### Monitoring

- Track test execution times to detect performance regressions
- Monitor test failure patterns to identify systemic issues
- Review test coverage to ensure comprehensive validation
- Update validation data based on real-world observations

This comprehensive testing suite ensures the Polygon POL Indexer is robust, performant, and reliable for production deployment.
