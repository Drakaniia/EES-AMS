import { test, expect } from '@playwright/test';

test.describe('Excel Import Tests', () => {
  test.beforeEach(async ({ page }) => {
    // Navigate to Students page
    await page.goto('http://localhost:5173');
    await page.click('[data-testid="nav-students"]');
  });

  test('should import students from Excel file', async ({ page }) => {
    // Mock file upload
    const fileInput = page.locator('input[type="file"]');
    
    // Create test file content (in real implementation, you'd upload actual file)
    const testFileContent = new Uint8Array([
      // Excel file content here
    ]);
    
    // Upload file
    await fileInput.setInputFiles({
      name: 'test-students.xlsx',
      mimeType: 'application/vnd.openxmlformats-officedocument.spreadsheetml.sheet',
      buffer: testFileContent,
    });
    
    // Wait for import to complete
    await page.waitForSelector('[data-testid="import-result"]');
    
    // Verify import result
    const successCount = await page.textContent('[data-testid="success-count"]');
    expect(successCount).toContain('1');
    
    // Verify student is in list
    await page.waitForSelector('[data-testid="student-item"]');
    const studentItems = await page.locator('[data-testid="student-item"]').count();
    expect(studentItems).toBeGreaterThan(0);
  });

  test('should handle invalid file format', async ({ page }) => {
    const fileInput = page.locator('input[type="file"]');
    
    // Upload invalid file
    await fileInput.setInputFiles({
      name: 'invalid.txt',
      mimeType: 'text/plain',
      buffer: new TextEncoder().encode('Invalid content'),
    });
    
    // Check error message
    const alertMessage = await page.waitForEvent('dialog');
    expect(alertMessage.message()).toContain('Please select an Excel file');
    await alertMessage.accept();
  });

  test('should display import errors', async ({ page }) => {
    // Test file with missing required fields
    const invalidFileContent = new Uint8Array([
      // Excel content with missing required fields
    ]);
    
    const fileInput = page.locator('input[type="file"]');
    await fileInput.setInputFiles({
      name: 'invalid-students.xlsx',
      mimeType: 'application/vnd.openxmlformats-officedocument.spreadsheetml.sheet',
      buffer: invalidFileContent,
    });
    
    // Wait for import result
    await page.waitForSelector('[data-testid="import-result"]');
    
    // Verify error display
    const errorCount = await page.textContent('[data-testid="error-count"]');
    expect(parseInt(errorCount || '0')).toBeGreaterThan(0);
    
    // Check error details
    const errorList = page.locator('[data-testid="error-list"]');
    await expect(errorList).toBeVisible();
  });

  test('should import to specific class', async ({ page }) => {
    // Create a test class first
    await page.click('[data-testid="nav-classes"]');
    await page.click('[data-testid="add-class-btn"]');
    await page.fill('[data-testid="class-name"]', 'Test Class');
    await page.click('[data-testid="save-class-btn"]');
    
    // Go back to students page
    await page.click('[data-testid="nav-students"]');
    
    // Upload file with class selection
    const fileInput = page.locator('input[type="file"]');
    await fileInput.setInputFiles({
      name: 'test-students.xlsx',
      mimeType: 'application/vnd.openxmlformats-officedocument.spreadsheetml.sheet',
      buffer: new Uint8Array([]),
    });
    
    // Select class from dropdown
    await page.selectOption('[data-testid="class-select"]', '1');
    
    // Import and verify
    await page.waitForSelector('[data-testid="import-result"]');
    
    // Verify student is assigned to class
    await page.click('[data-testid="nav-classes"]');
    await page.click('[data-testid="class-1"]');
    const studentsInClass = await page.locator('[data-testid="student-item"]').count();
    expect(studentsInClass).toBeGreaterThan(0);
  });
});

test.describe('Firebase Sync Integration Tests', () => {
  test.beforeEach(async ({ page }) => {
    // Configure mock Firebase in environment
    await page.goto('http://localhost:5173');
  });

  test('should sync data to Firebase', async ({ page }) => {
    // Enable Firebase sync in settings
    await page.click('[data-testid="nav-settings"]');
    await page.check('[data-testid="firebase-sync-enabled"]');
    
    // Save settings
    await page.click('[data-testid="save-settings-btn"]');
    
    // Wait for sync to complete
    await page.waitForSelector('[data-testid="sync-status"]');
    const syncStatus = await page.textContent('[data-testid="sync-status"]');
    expect(syncStatus).toContain('Synced');
  });

  test('should handle sync conflicts', async ({ page }) => {
    // Create conflict scenario
    await page.click('[data-testid="nav-students"]');
    await page.click('[data-testid="edit-student-1"]');
    await page.fill('[data-testid="student-name"]', 'Updated Name');
    await page.click('[data-testid="save-student-btn"]');
    
    // Simulate remote conflict
    // In real implementation, you'd mock Firebase responses
    
    // Check conflict resolution UI
    await page.waitForSelector('[data-testid="conflict-dialog"]');
    await expect(page.locator('[data-testid="conflict-dialog"]')).toBeVisible();
    
    // Resolve conflict (choose recent version)
    await page.click('[data-testid="resolve-conflict-recent"]');
    await page.click('[data-testid="confirm-resolution"]');
    
    // Verify resolution
    await expect(page.locator('[data-testid="conflict-dialog"]')).not.toBeVisible();
    const finalName = await page.textContent('[data-testid="student-1-name"]');
    expect(finalName).toContain('Updated Name');
  });
});

test.describe('Hybrid Storage Tests', () => {
  test('should work offline and sync when online', async ({ page }) => {
    // Set to offline mode
    await page.context().setOffline(true);
    
    // Perform actions offline
    await page.goto('http://localhost:5173');
    await page.click('[data-testid="nav-students"]');
    await page.click('[data-testid="add-student-btn"]');
    await page.fill('[data-testid="student-name"]', 'Offline Student');
    await page.click('[data-testid="save-student-btn"]');
    
    // Verify student exists locally
    await expect(page.locator('[data-testid="offline-student"]')).toBeVisible();
    
    // Go back online
    await page.context().setOffline(false);
    
    // Wait for auto-sync
    await page.waitForSelector('[data-testid="sync-indicator"]');
    const syncStatus = await page.textContent('[data-testid="sync-indicator"]');
    expect(syncStatus).toContain('Syncing');
    
    // Wait for sync to complete
    await page.waitForSelector('[data-testid="sync-indicator"]:has-text("Synced")');
  });

  test('should maintain data consistency across multiple storage systems', async ({ page }) => {
    // Configure hybrid storage
    await page.goto('http://localhost:5173/settings');
    await page.check('[data-testid="local-storage"]');
    await page.check('[data-testid="google-drive-storage"]');
    await page.check('[data-testid="firebase-storage"]');
    await page.click('[data-testid="save-settings"]');
    
    // Create data
    await page.click('[data-testid="nav-students"]');
    await page.click('[data-testid="add-student-btn"]');
    await page.fill('[data-testid="student-name"]', 'Consistency Test');
    await page.click('[data-testid="save-student-btn"]');
    
    // Wait for all syncs to complete
    await page.waitForSelector('[data-testid="all-systems-synced"]');
    
    // Verify data in all systems (would need mock verification)
    await page.reload();
    await expect(page.locator('[data-testid="consistency-test"]')).toBeVisible();
  });
});

test.describe('Performance Tests', () => {
  test('should handle large Excel file imports efficiently', async ({ page }) => {
    // Large file upload
    const largeFileContent = new Uint8Array(new Array(1000000).fill(0));
    const fileInput = page.locator('input[type="file"]');
    
    const startTime = Date.now();
    
    await fileInput.setInputFiles({
      name: 'large-student-list.xlsx',
      mimeType: 'application/vnd.openxmlformats-officedocument.spreadsheetml.sheet',
      buffer: largeFileContent,
    });
    
    // Wait for processing with timeout
    try {
      await page.waitForSelector('[data-testid="import-complete"]', {
        timeout: 60000,
      });
      
      const endTime = Date.now();
      const importDuration = endTime - startTime;
      
      // Should complete within 30 seconds (adjust based on requirements)
      expect(importDuration).toBeLessThan(30000);
    } catch {
      const timeElapsed = Date.now() - startTime;
      test.fail(`Import took too long: ${timeElapsed}ms`);
    }
  });

  test('should handle concurrent sync operations', async ({ page }) => {
    const startTime = Date.now();
    
    // Initiate multiple sync operations
    await page.goto('http://localhost:5173');
    
    // Simulate concurrent operations
    const promises = [
      page.click('[data-testid="sync-students"]'),
      page.click('[data-testid="sync-classes"]'),
      page.click('[data-testid="sync-attendance"]'),
    ];
    
    await Promise.all(promises);
    
    // Wait for all operations to complete
    await page.waitForSelector('[data-testid="all-sync-complete"]', {
      timeout: 45000,
    });
    
    const endTime = Date.now();
    const syncDuration = endTime - startTime;
    
    // Concurrent operations should be faster than sequential
    expect(syncDuration).toBeLessThan(30000);
  });
});