#!/bin/bash

# Comprehensive Test Runner for Polygon POL Indexer
# This script runs all types of tests: unit, integration, performance, and validation

set -e

echo "🚀 Starting Comprehensive Test Suite for Polygon POL Indexer"
echo "============================================================"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Function to print colored output
print_status() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Check if cargo is available
if ! command -v cargo &> /dev/null; then
    print_error "Cargo is not installed or not in PATH"
    exit 1
fi

# Create test results directory
mkdir -p test_results
TIMESTAMP=$(date +"%Y%m%d_%H%M%S")
RESULTS_DIR="test_results/run_$TIMESTAMP"
mkdir -p "$RESULTS_DIR"

print_status "Test results will be saved to: $RESULTS_DIR"

# Function to run tests and capture output
run_test_suite() {
    local test_name=$1
    local test_command=$2
    local output_file="$RESULTS_DIR/${test_name}_output.log"
    
    print_status "Running $test_name..."
    
    if eval "$test_command" > "$output_file" 2>&1; then
        print_success "$test_name completed successfully"
        return 0
    else
        print_error "$test_name failed"
        echo "Error output:"
        tail -20 "$output_file"
        return 1
    fi
}

# Initialize test results tracking
TOTAL_TESTS=0
PASSED_TESTS=0
FAILED_TESTS=0

# 1. Unit Tests
print_status "Phase 1: Running Unit Tests"
echo "----------------------------"

TOTAL_TESTS=$((TOTAL_TESTS + 1))
if run_test_suite "unit_tests" "cargo test --lib"; then
    PASSED_TESTS=$((PASSED_TESTS + 1))
else
    FAILED_TESTS=$((FAILED_TESTS + 1))
fi

# 2. Integration Tests (existing)
print_status "Phase 2: Running Existing Integration Tests"
echo "-------------------------------------------"

TOTAL_TESTS=$((TOTAL_TESTS + 1))
if run_test_suite "existing_integration_tests" "cargo test --test integration_block_monitoring --test integration_block_processing"; then
    PASSED_TESTS=$((PASSED_TESTS + 1))
else
    FAILED_TESTS=$((FAILED_TESTS + 1))
fi

# 3. End-to-End Workflow Tests
print_status "Phase 3: Running End-to-End Workflow Tests"
echo "-------------------------------------------"

TOTAL_TESTS=$((TOTAL_TESTS + 1))
if run_test_suite "end_to_end_tests" "cargo test --test end_to_end_workflow"; then
    PASSED_TESTS=$((PASSED_TESTS + 1))
else
    FAILED_TESTS=$((FAILED_TESTS + 1))
fi

# 4. Performance Tests
print_status "Phase 4: Running Performance Tests"
echo "----------------------------------"

TOTAL_TESTS=$((TOTAL_TESTS + 1))
if run_test_suite "performance_tests" "cargo test --test performance_tests --release"; then
    PASSED_TESTS=$((PASSED_TESTS + 1))
else
    FAILED_TESTS=$((FAILED_TESTS + 1))
fi

# 5. Network Failure and Recovery Tests
print_status "Phase 5: Running Network Failure and Recovery Tests"
echo "---------------------------------------------------"

TOTAL_TESTS=$((TOTAL_TESTS + 1))
if run_test_suite "network_failure_tests" "cargo test --test network_failure_recovery"; then
    PASSED_TESTS=$((PASSED_TESTS + 1))
else
    FAILED_TESTS=$((FAILED_TESTS + 1))
fi

# 6. Validation Tests
print_status "Phase 6: Running Validation Tests"
echo "---------------------------------"

TOTAL_TESTS=$((TOTAL_TESTS + 1))
if run_test_suite "validation_tests" "cargo test --test validation_tests"; then
    PASSED_TESTS=$((PASSED_TESTS + 1))
else
    FAILED_TESTS=$((FAILED_TESTS + 1))
fi

# 7. Real Polygon Testnet Tests (optional, requires network)
print_status "Phase 7: Running Real Polygon Testnet Tests (Optional)"
echo "------------------------------------------------------"

print_warning "These tests require network connectivity and may be slow"
read -p "Do you want to run real testnet tests? (y/N): " -n 1 -r
echo

if [[ $REPLY =~ ^[Yy]$ ]]; then
    TOTAL_TESTS=$((TOTAL_TESTS + 1))
    if run_test_suite "testnet_integration_tests" "cargo test --test integration_polygon_testnet -- --ignored"; then
        PASSED_TESTS=$((PASSED_TESTS + 1))
    else
        FAILED_TESTS=$((FAILED_TESTS + 1))
        print_warning "Testnet tests failed - this may be due to network issues"
    fi
else
    print_status "Skipping real testnet tests"
fi

# 8. Benchmarks (optional)
print_status "Phase 8: Running Performance Benchmarks (Optional)"
echo "--------------------------------------------------"

read -p "Do you want to run performance benchmarks? (y/N): " -n 1 -r
echo

if [[ $REPLY =~ ^[Yy]$ ]]; then
    print_status "Running Criterion benchmarks..."
    if cargo bench > "$RESULTS_DIR/benchmarks_output.log" 2>&1; then
        print_success "Benchmarks completed successfully"
        print_status "Benchmark results saved to target/criterion/"
    else
        print_error "Benchmarks failed"
        tail -20 "$RESULTS_DIR/benchmarks_output.log"
    fi
else
    print_status "Skipping benchmarks"
fi

# 9. Code Coverage (optional, requires cargo-tarpaulin)
print_status "Phase 9: Code Coverage Analysis (Optional)"
echo "-----------------------------------------"

if command -v cargo-tarpaulin &> /dev/null; then
    read -p "Do you want to run code coverage analysis? (y/N): " -n 1 -r
    echo
    
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        print_status "Running code coverage analysis..."
        if cargo tarpaulin --out Html --output-dir "$RESULTS_DIR" > "$RESULTS_DIR/coverage_output.log" 2>&1; then
            print_success "Code coverage analysis completed"
            print_status "Coverage report saved to $RESULTS_DIR/tarpaulin-report.html"
        else
            print_error "Code coverage analysis failed"
            tail -20 "$RESULTS_DIR/coverage_output.log"
        fi
    else
        print_status "Skipping code coverage analysis"
    fi
else
    print_warning "cargo-tarpaulin not found. Install with: cargo install cargo-tarpaulin"
fi

# Generate summary report
print_status "Generating Test Summary Report"
echo "==============================="

SUMMARY_FILE="$RESULTS_DIR/test_summary.txt"

cat > "$SUMMARY_FILE" << EOF
Polygon POL Indexer - Comprehensive Test Results
===============================================

Test Run: $TIMESTAMP
Total Test Suites: $TOTAL_TESTS
Passed: $PASSED_TESTS
Failed: $FAILED_TESTS
Success Rate: $(( PASSED_TESTS * 100 / TOTAL_TESTS ))%

Test Suite Details:
------------------
EOF

# Add individual test results
for log_file in "$RESULTS_DIR"/*_output.log; do
    if [ -f "$log_file" ]; then
        test_name=$(basename "$log_file" _output.log)
        if grep -q "test result: ok" "$log_file" 2>/dev/null; then
            echo "✅ $test_name: PASSED" >> "$SUMMARY_FILE"
        else
            echo "❌ $test_name: FAILED" >> "$SUMMARY_FILE"
        fi
    fi
done

cat >> "$SUMMARY_FILE" << EOF

Performance Metrics:
-------------------
EOF

# Add performance metrics if available
if [ -f "$RESULTS_DIR/performance_tests_output.log" ]; then
    echo "Performance test results:" >> "$SUMMARY_FILE"
    grep -E "(transfers/second|queries/second|blocks/second)" "$RESULTS_DIR/performance_tests_output.log" | head -10 >> "$SUMMARY_FILE" 2>/dev/null || true
fi

# Display final results
echo
echo "🎯 COMPREHENSIVE TEST RESULTS"
echo "============================="
echo "Total Test Suites: $TOTAL_TESTS"
echo "Passed: $PASSED_TESTS"
echo "Failed: $FAILED_TESTS"

if [ $FAILED_TESTS -eq 0 ]; then
    print_success "All test suites passed! 🎉"
    echo
    print_status "The Polygon POL Indexer has been comprehensively validated:"
    echo "  ✅ Unit tests verify individual component functionality"
    echo "  ✅ Integration tests validate component interactions"
    echo "  ✅ End-to-end tests confirm complete workflow"
    echo "  ✅ Performance tests ensure acceptable throughput"
    echo "  ✅ Network failure tests verify resilience"
    echo "  ✅ Validation tests confirm data accuracy"
    
    if [ -f "$RESULTS_DIR/testnet_integration_tests_output.log" ]; then
        echo "  ✅ Real testnet integration validated"
    fi
    
    echo
    print_status "The system is ready for deployment! 🚀"
else
    print_error "Some test suites failed. Please review the logs in $RESULTS_DIR"
    echo
    print_status "Failed test suites need to be addressed before deployment."
fi

echo
print_status "Detailed results and logs available in: $RESULTS_DIR"
print_status "Summary report: $SUMMARY_FILE"

# Display summary
cat "$SUMMARY_FILE"

# Exit with appropriate code
if [ $FAILED_TESTS -eq 0 ]; then
    exit 0
else
    exit 1
fi