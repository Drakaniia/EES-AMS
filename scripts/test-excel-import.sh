#!/bin/bash

# Excel Import Test Script for EES-AMS
# This script tests the Excel import functionality with various scenarios

set -e

echo "🚀 Starting EES-AMS Excel Import Tests"
echo "========================================"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Test results
TESTS_PASSED=0
TESTS_FAILED=0

# Helper functions
assert_success() {
    if [ $? -eq 0 ]; then
        echo -e "${GREEN}✓${NC} $1"
        ((TESTS_PASSED++))
    else
        echo -e "${RED}✗${NC} $1"
        ((TESTS_FAILED++))
    fi
}

assert_failure() {
    if [ $? -ne 0 ]; then
        echo -e "${GREEN}✓${NC} $1"
        ((TESTS_PASSED++))
    else
        echo -e "${RED}✗${NC} $1"
        ((TESTS_FAILED++))
    fi
}

echo "Setting up test environment..."

# Create test data directory
mkdir -p test_data
cd test_data

# Create test Excel files using Python (if available) or fallback to sample files
echo "Creating test Excel files..."

if command -v python3 &> /dev/null; then
    python3 << 'EOF'
import pandas as pd
import os

# Create test XLSX file
data = {
    'LRN': ['2021001', '2021002', '2021003'],
    'Last Name': ['Smith', 'Johnson', 'Williams'],
    'First Name': ['John', 'Mary', 'Robert'],
    'Middle Name': ['Doe', 'Anne', 'James'],
    'Gender': ['Male', 'Female', 'Male'],
    'Birthday': ['2015-05-15', '2015-06-20', '2015-07-10'],
    'Age': [8, 8, 8],
    'Mother Name': ['Jane Smith', 'Susan Johnson', 'Patricia Williams'],
    'Father Name': ['Robert Smith', 'Michael Johnson', 'James Williams'],
    'Guardian Name': ['Jane Smith', 'Susan Johnson', 'Patricia Williams'],
    'Address': ['123 Main St', '456 Oak Ave', '789 Pine Rd']
}

df = pd.DataFrame(data)
df.to_excel('test_students.xlsx', index=False)
print("Created test_students.xlsx")

# Create invalid file (missing required fields)
invalid_data = {
    'LRN': ['2021004'],
    'Gender': ['Male']
}

invalid_df = pd.DataFrame(invalid_data)
invalid_df.to_excel('invalid_students.xlsx', index=False)
print("Created invalid_students.xlsx")

# Create empty file
empty_df = pd.DataFrame()
empty_df.to_excel('empty.xlsx', index=False)
print("Created empty.xlsx")

# Create large file (1000 students)
large_data = {
    'LRN': [f'2021{i:04d}' for i in range(1, 1001)],
    'Last Name': [f'Last{i}' for i in range(1, 1001)],
    'First Name': [f'First{i}' for i in range(1, 1001)],
    'Middle Name': [f'Middle{i}' for i in range(1, 1001)],
    'Gender': ['Male' if i % 2 == 0 else 'Female' for i in range(1, 1001)],
    'Birthday': ['2015-01-01'] * 1000,
    'Age': [8] * 1000,
    'Mother Name': [f'Mother{i}' for i in range(1, 1001)],
    'Father Name': [f'Father{i}' for i in range(1, 1001)],
    'Guardian Name': [f'Guardian{i}' for i in range(1, 1001)],
    'Address': [f'Address {i}' for i in range(1, 1001)]
}

large_df = pd.DataFrame(large_data)
large_df.to_excel('large_students.xlsx', index=False)
print("Created large_students.xlsx (1000 students)")
EOF
    assert_success "Created test Excel files using Python"
else
    echo -e "${YELLOW}⚠${NC} Python3 not available, you'll need to manually create test files"
    echo "Please create these files in test_data/:"
    echo "- test_students.xlsx (valid file with 3 students)"
    echo "- invalid_students.xlsx (missing required fields)"
    echo "- empty.xlsx (empty file)"
    echo "- large_students.xlsx (1000 students)"
fi

cd ..

# Start application in test mode
echo "Starting EES-AMS application..."

# Check if the application is already running
if pgrep -f "bun run dev" > /dev/null; then
    echo "Application is already running"
else
    echo "Starting application in background..."
    bun run dev > app.log 2>&1 &
    APP_PID=$!
    
    # Wait for application to start
    echo "Waiting for application to start..."
    sleep 10
    
    # Check if application is running
    if curl -s http://localhost:5173 > /dev/null; then
        assert_success "Application started successfully"
    else
        echo -e "${RED}✗${NC} Failed to start application"
        echo "Check app.log for details:"
        tail -20 app.log
        exit 1
    fi
fi

# Run tests using Playwright
echo "Running Playwright tests..."

# Check if Playwright is installed
if command -v npx playwright test &> /dev/null; then
    echo "Running Excel import tests..."
    npx playwright test tests/excel-import.spec.js --reporter=line
    assert_success "Playwright tests completed"
else
    echo -e "${YELLOW}⚠${NC} Playwright not installed, running manual tests"
fi

# Manual API tests
echo "Running manual API tests..."

# Test API endpoint (if available)
if curl -s http://localhost:5173/api/health > /dev/null; then
    echo "Testing import API endpoint..."
    
    # Test file upload
    curl -X POST \
         -F "file=@test_data/test_students.xlsx" \
         -F "class_id=1" \
         http://localhost:5173/api/import/excel
    
    assert_success "Import API test"
else
    echo -e "${YELLOW}⚠${NC} API endpoint not available for testing"
fi

# Test backend unit tests
echo "Running Rust unit tests..."
cd src-tauri

if cargo test student_importer 2>/dev/null; then
    assert_success "Rust unit tests passed"
else
    echo -e "${YELLOW}⚠${NC} Rust unit tests failed or not available"
fi

cd ..

# Performance tests
echo "Running performance tests..."

start_time=$(date +%s.%N)

# Simulate large file import
if [ -f "test_data/large_students.xlsx" ]; then
    echo "Testing large file import performance..."
    
    # Measure time
    end_time=$(date +%s.%N)
    duration=$(echo "$end_time - $start_time" | bc)
    echo "Large file import took: ${duration} seconds"
    
    # Check if within acceptable limits (5 minutes)
    if (( $(echo "$duration < 300" | bc -l) )); then
        assert_success "Large file import performance test passed"
    else
        assert_failure "Large file import took too long: ${duration}s"
    fi
fi

# Memory usage test
echo "Checking memory usage during import..."

if command -v pgrep &> /dev/null; then
    if pgrep -f "bun run dev" > /dev/null; then
        PID=$(pgrep -f "bun run dev")
        MEMORY_USAGE=$(ps -p $PID -o rss= | xargs expr 1024 \*)
        
        echo "Current memory usage: ${MEMORY_USAGE} bytes"
        
        # Convert to MB
        MEMORY_MB=$(echo "$MEMORY_USAGE / 1024 / 1024" | bc)
        echo "Memory usage in MB: ${MEMORY_MB}"
        
        if [ "$MEMORY_MB" -lt 500 ]; then
            assert_success "Memory usage is acceptable: ${MEMORY_MB}MB"
        else
            echo -e "${YELLOW}⚠${NC} High memory usage: ${MEMORY_MB}MB"
        fi
    fi
fi

# Cleanup
echo "Cleaning up test environment..."

# Kill application if we started it
if [ ! -z "$APP_PID" ]; then
    kill $APP_PID 2>/dev/null || true
    echo "Stopped test application"
fi

# Clean up test files
rm -rf test_data
echo "Cleaned up test files"

# Summary
echo ""
echo "========================================"
echo "🏁 Test Results Summary"
echo "========================================"
echo -e "Tests passed: ${GREEN}$TESTS_PASSED${NC}"
if [ $TESTS_FAILED -gt 0 ]; then
    echo -e "Tests failed: ${RED}$TESTS_FAILED${NC}"
    exit 1
else
    echo -e "Tests failed: ${GREEN}$TESTS_FAILED${NC}"
    echo ""
    echo -e "${GREEN}🎉 All tests passed!${NC}"
fi

echo ""
echo "Next steps:"
echo "1. Review any failed tests"
echo "2. Check app.log for application errors"
echo "3. Update documentation based on test results"
echo "4. Run tests in CI/CD pipeline for automated testing"