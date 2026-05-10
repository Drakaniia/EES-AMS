# Unit Tests

## Domain Models Testing
- Model validation: Test UUID parsing, serialization/deserialization
- Default values: Verify Settings::default() and ID generation
- Business logic: Test attendance type conversions, timestamp handling

## Repository Layer Testing
- CRUD operations: Test all create, read, update, delete operations for each repository
- Edge cases: Test duplicate card serials, foreign key constraints, unique constraints
- Query optimization: Verify indexed queries perform correctly
- Data integrity: Test cascading deletes and referential integrity

## Error Handling Testing
- Custom error types: Test all AppError variants
- Error propagation: Verify proper error conversion from database to domain layer
- Recovery scenarios: Test behavior after database corruption or connection loss

# Integration Tests

## Database Integration
- Migration testing: Test schema migrations from v0 to v1 and future versions
- Transaction handling: Test rollback scenarios and concurrent access
- Connection pooling: Test pool exhaustion and recovery

## Command Integration
- Tauri commands: Test all 22+ commands with valid/invalid inputs
- State management: Test shared state across multiple command invocations
- Async operations: Test background server startup and NFC scanning

# Mock Testing

## Hardware Simulation
- NFC reader mocking: Test reader connection/disconnection scenarios
- Card reading simulation: Test various card data formats and error conditions
- USB device mocking: Test PC/SC library interactions

## External Dependencies
- Network services: Mock HTTP server endpoints and IP address detection
- File system: Test database file creation, permissions, and corruption scenarios

# Performance Tests

## Database Performance
- Large datasets: Test with thousands of students and events
- Query performance: Benchmark complex queries with indexes
- Concurrent access: Test multiple simultaneous database operations

## Memory Management
- Connection leaks: Verify proper connection cleanup
- Memory usage: Monitor memory consumption during extended operations
- Resource limits: Test behavior under resource constraints

# End-to-End Tests

## User Workflows
- Student lifecycle: Create → Update → Delete with attendance tracking
- Class management: Complete class setup with student assignments
- Import/Export: Test data migration and backup/restore scenarios

## Error Recovery
- Database corruption: Test recovery from damaged database files
- Hardware failures: Test behavior when NFC reader disconnects mid-operation
- Network issues: Test HTTP server resilience under network stress

# Security Tests

## Input Validation
- SQL injection: Test all database queries against injection attacks
- Data sanitization: Verify all user inputs are properly validated
- Path traversal: Test file operations against directory traversal attacks

## Access Control
- Command permissions: Test unauthorized command access
- Data isolation: Verify user data cannot access other users' data

# Recommended Test Structure

```
src-tauri/tests/
├── unit/
│   ├── models/
│   ├── repositories/
│   └── services/
├── integration/
│   ├── database/
│   ├── commands/
│   └── nfc/
├── performance/
├── security/
└── e2e/
```