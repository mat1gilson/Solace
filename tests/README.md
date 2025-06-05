# Solace Protocol Test Suite

Comprehensive testing framework for the Solace Protocol autonomous agent commerce system.

## Test Categories

### Unit Tests
- **Framework Tests** (`framework/src/`) - Core library unit tests
- **ACP Tests** (`acp/src/`) - Protocol messaging unit tests
- **AI Tests** (`ai/src/`) - AI decision-making unit tests

### Integration Tests
- **Agent Lifecycle Tests** - Complete agent creation, operation, and cleanup
- **Transaction Flow Tests** - End-to-end transaction processing
- **Multi-Agent Coordination** - Complex scenarios with multiple agents
- **Network Communication Tests** - ACP messaging and discovery

### Load & Performance Tests
- **Throughput Tests** - Transaction processing capacity
- **Latency Tests** - Response time measurements
- **Scalability Tests** - Performance under high agent counts
- **Memory Tests** - Resource usage optimization

### End-to-End Tests
- **Real Network Tests** - Testing on Solana devnet
- **Cross-Platform Tests** - SDK compatibility across languages
- **Security Tests** - Cryptographic verification and attack resistance

## Running Tests

### Quick Test Suite
```bash
# Run all unit tests
cargo test

# Run integration tests
cargo test --test integration_tests

# Run with coverage
cargo tarpaulin --out Html
```

### Load Testing
```bash
# Run performance benchmarks
cargo bench

# Run load tests
cargo run --bin load_test -- --agents 100 --transactions 1000

# Run stress tests
cargo run --bin stress_test -- --duration 300s
```

### E2E Testing
```bash
# Start test validator
solana-test-validator &

# Run end-to-end tests
cargo run --bin e2e_test

# Clean up
pkill solana-test-validator
```

## Test Configuration

### Environment Variables
- `SOLACE_TEST_RPC` - Custom RPC endpoint for testing
- `SOLACE_TEST_LOG_LEVEL` - Logging level (debug, info, warn, error)
- `SOLACE_TEST_PARALLEL` - Enable parallel test execution
- `SOLACE_TEST_TIMEOUT` - Test timeout in seconds

### Test Data
- `test_data/` - Static test datasets
- `fixtures/` - Reusable test fixtures
- `mocks/` - Mock services and responses

## Continuous Integration

Tests are automatically run on:
- All pull requests
- Main branch commits
- Nightly performance benchmarks
- Weekly security audits

### Test Reports
- Coverage reports available at `/coverage/`
- Performance benchmarks at `/benchmarks/`
- Security scan results at `/security/`

## Writing Tests

### Guidelines
1. **Use descriptive test names** - Clearly indicate what is being tested
2. **Follow AAA pattern** - Arrange, Act, Assert
3. **Test edge cases** - Include boundary conditions and error cases
4. **Use property-based testing** - For complex invariants
5. **Mock external dependencies** - Ensure tests are deterministic

### Example Test Structure
```rust
#[tokio::test]
async fn test_agent_transaction_lifecycle() {
    // Arrange
    let agent = create_test_agent().await;
    let transaction_request = create_test_request();
    
    // Act
    let result = agent.process_transaction(transaction_request).await;
    
    // Assert
    assert!(result.is_ok());
    assert_eq!(result.unwrap().status, TransactionStatus::Completed);
}
```

## Test Utilities

### TestAgentFactory
Helper for creating test agents with predefined configurations:
- `create_basic_config()` - Standard test agent
- `create_trading_agent()` - Specialized trading agent
- `create_analysis_agent()` - Data analysis specialist

### TestEnvironment
Managed test environment for multi-agent scenarios:
- Agent lifecycle management
- Network simulation
- Resource cleanup

### Mock Services
- **MockSolanaRPC** - Simulated blockchain interactions
- **MockNetworkLayer** - Controlled network conditions
- **MockTimeService** - Deterministic time progression

## Performance Baselines

### Target Metrics
- **Agent Creation**: < 100ms per agent
- **Transaction Processing**: < 1s for simple transactions
- **Network Latency**: < 50ms between local agents
- **Memory Usage**: < 10MB per agent baseline

### Benchmark Categories
- `agent_performance` - Agent operation benchmarks
- `transaction_throughput` - Transaction processing speed
- `network_latency` - Communication performance
- `memory_usage` - Resource consumption

## Debugging Tests

### Logging
```bash
# Enable debug logging
RUST_LOG=debug cargo test

# Test-specific logging
RUST_LOG=solace_protocol::agent=trace cargo test test_agent_lifecycle
```

### Test Isolation
- Each test runs in isolated environment
- Temporary directories for file operations
- Random ports for network tests
- Fresh agent instances per test

## Contributing to Tests

When adding new features:
1. **Add unit tests** for new functions/methods
2. **Update integration tests** for API changes
3. **Add performance tests** for performance-critical code
4. **Update documentation** for test procedures

See [CONTRIBUTING.md](../CONTRIBUTING.md) for detailed guidelines.

---

For questions about testing, please:
- Check existing test examples
- Open an issue for test-related bugs
- Join our Discord for testing discussions 