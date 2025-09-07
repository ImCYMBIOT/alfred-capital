@echo off
setlocal enabledelayedexpansion

REM Comprehensive Test Runner for Polygon POL Indexer (Windows)
REM This script runs all types of tests: unit, integration, performance, and validation

echo 🚀 Starting Comprehensive Test Suite for Polygon POL Indexer
echo ============================================================

REM Check if cargo is available
where cargo >nul 2>nul
if %errorlevel% neq 0 (
    echo [ERROR] Cargo is not installed or not in PATH
    exit /b 1
)

REM Create test results directory
if not exist test_results mkdir test_results
for /f "tokens=2 delims==" %%a in ('wmic OS Get localdatetime /value') do set "dt=%%a"
set "TIMESTAMP=%dt:~0,4%%dt:~4,2%%dt:~6,2%_%dt:~8,2%%dt:~10,2%%dt:~12,2%"
set "RESULTS_DIR=test_results\run_%TIMESTAMP%"
mkdir "%RESULTS_DIR%"

echo [INFO] Test results will be saved to: %RESULTS_DIR%

REM Initialize test results tracking
set TOTAL_TESTS=0
set PASSED_TESTS=0
set FAILED_TESTS=0

echo [INFO] Phase 1: Running Unit Tests
echo ----------------------------
set /a TOTAL_TESTS+=1
cargo test --lib > "%RESULTS_DIR%\unit_tests_output.log" 2>&1
if %errorlevel% equ 0 (
    echo [SUCCESS] Unit tests completed successfully
    set /a PASSED_TESTS+=1
) else (
    echo [ERROR] Unit tests failed
    set /a FAILED_TESTS+=1
)

echo [INFO] Phase 2: Running Existing Integration Tests
echo -------------------------------------------
set /a TOTAL_TESTS+=1
cargo test --test integration_block_monitoring --test integration_block_processing > "%RESULTS_DIR%\existing_integration_tests_output.log" 2>&1
if %errorlevel% equ 0 (
    echo [SUCCESS] Existing integration tests completed successfully
    set /a PASSED_TESTS+=1
) else (
    echo [ERROR] Existing integration tests failed
    set /a FAILED_TESTS+=1
)

echo [INFO] Phase 3: Running End-to-End Workflow Tests
echo -------------------------------------------
set /a TOTAL_TESTS+=1
cargo test --test end_to_end_workflow > "%RESULTS_DIR%\end_to_end_tests_output.log" 2>&1
if %errorlevel% equ 0 (
    echo [SUCCESS] End-to-end tests completed successfully
    set /a PASSED_TESTS+=1
) else (
    echo [ERROR] End-to-end tests failed
    set /a FAILED_TESTS+=1
)

echo [INFO] Phase 4: Running Performance Tests
echo ----------------------------------
set /a TOTAL_TESTS+=1
cargo test --test performance_tests --release > "%RESULTS_DIR%\performance_tests_output.log" 2>&1
if %errorlevel% equ 0 (
    echo [SUCCESS] Performance tests completed successfully
    set /a PASSED_TESTS+=1
) else (
    echo [ERROR] Performance tests failed
    set /a FAILED_TESTS+=1
)

echo [INFO] Phase 5: Running Network Failure and Recovery Tests
echo ---------------------------------------------------
set /a TOTAL_TESTS+=1
cargo test --test network_failure_recovery > "%RESULTS_DIR%\network_failure_tests_output.log" 2>&1
if %errorlevel% equ 0 (
    echo [SUCCESS] Network failure tests completed successfully
    set /a PASSED_TESTS+=1
) else (
    echo [ERROR] Network failure tests failed
    set /a FAILED_TESTS+=1
)

echo [INFO] Phase 6: Running Validation Tests
echo ---------------------------------
set /a TOTAL_TESTS+=1
cargo test --test validation_tests > "%RESULTS_DIR%\validation_tests_output.log" 2>&1
if %errorlevel% equ 0 (
    echo [SUCCESS] Validation tests completed successfully
    set /a PASSED_TESTS+=1
) else (
    echo [ERROR] Validation tests failed
    set /a FAILED_TESTS+=1
)

echo [INFO] Phase 7: Running Real Polygon Testnet Tests (Optional)
echo ------------------------------------------------------
echo [WARNING] These tests require network connectivity and may be slow
set /p "REPLY=Do you want to run real testnet tests? (y/N): "
if /i "%REPLY%"=="y" (
    set /a TOTAL_TESTS+=1
    cargo test --test integration_polygon_testnet -- --ignored > "%RESULTS_DIR%\testnet_integration_tests_output.log" 2>&1
    if !errorlevel! equ 0 (
        echo [SUCCESS] Testnet integration tests completed successfully
        set /a PASSED_TESTS+=1
    ) else (
        echo [ERROR] Testnet integration tests failed
        echo [WARNING] Testnet tests failed - this may be due to network issues
        set /a FAILED_TESTS+=1
    )
) else (
    echo [INFO] Skipping real testnet tests
)

echo [INFO] Phase 8: Running Performance Benchmarks (Optional)
echo --------------------------------------------------
set /p "REPLY=Do you want to run performance benchmarks? (y/N): "
if /i "%REPLY%"=="y" (
    echo [INFO] Running Criterion benchmarks...
    cargo bench > "%RESULTS_DIR%\benchmarks_output.log" 2>&1
    if !errorlevel! equ 0 (
        echo [SUCCESS] Benchmarks completed successfully
        echo [INFO] Benchmark results saved to target\criterion\
    ) else (
        echo [ERROR] Benchmarks failed
    )
) else (
    echo [INFO] Skipping benchmarks
)

REM Generate summary report
echo [INFO] Generating Test Summary Report
echo ===============================

set "SUMMARY_FILE=%RESULTS_DIR%\test_summary.txt"

echo Polygon POL Indexer - Comprehensive Test Results > "%SUMMARY_FILE%"
echo =============================================== >> "%SUMMARY_FILE%"
echo. >> "%SUMMARY_FILE%"
echo Test Run: %TIMESTAMP% >> "%SUMMARY_FILE%"
echo Total Test Suites: %TOTAL_TESTS% >> "%SUMMARY_FILE%"
echo Passed: %PASSED_TESTS% >> "%SUMMARY_FILE%"
echo Failed: %FAILED_TESTS% >> "%SUMMARY_FILE%"

REM Calculate success rate
set /a SUCCESS_RATE=PASSED_TESTS*100/TOTAL_TESTS
echo Success Rate: %SUCCESS_RATE%%% >> "%SUMMARY_FILE%"
echo. >> "%SUMMARY_FILE%"

REM Display final results
echo.
echo 🎯 COMPREHENSIVE TEST RESULTS
echo =============================
echo Total Test Suites: %TOTAL_TESTS%
echo Passed: %PASSED_TESTS%
echo Failed: %FAILED_TESTS%

if %FAILED_TESTS% equ 0 (
    echo [SUCCESS] All test suites passed! 🎉
    echo.
    echo [INFO] The Polygon POL Indexer has been comprehensively validated:
    echo   ✅ Unit tests verify individual component functionality
    echo   ✅ Integration tests validate component interactions
    echo   ✅ End-to-end tests confirm complete workflow
    echo   ✅ Performance tests ensure acceptable throughput
    echo   ✅ Network failure tests verify resilience
    echo   ✅ Validation tests confirm data accuracy
    echo.
    echo [INFO] The system is ready for deployment! 🚀
) else (
    echo [ERROR] Some test suites failed. Please review the logs in %RESULTS_DIR%
    echo.
    echo [INFO] Failed test suites need to be addressed before deployment.
)

echo.
echo [INFO] Detailed results and logs available in: %RESULTS_DIR%
echo [INFO] Summary report: %SUMMARY_FILE%

REM Display summary
type "%SUMMARY_FILE%"

REM Exit with appropriate code
if %FAILED_TESTS% equ 0 (
    exit /b 0
) else (
    exit /b 1
)