import 'dart:io';
import 'dart:convert';
import 'package:path_provider/path_provider.dart';
import 'package:intl/intl.dart';

/// Service for managing data backup and restore
class BackupService {
  /// Create a backup of all application data
  Future<File> createBackup() async {
    try {
      final timestamp = DateFormat('yyyyMMdd_HHmmss').format(DateTime.now());
      final fileName = 'attendease_backup_$timestamp.json';

      // Get documents directory
      final directory = await getApplicationDocumentsDirectory();
      final backupDir = Directory('${directory.path}/backups');

      // Create backups directory if it doesn't exist
      if (!await backupDir.exists()) {
        await backupDir.create(recursive: true);
      }

      final backupFile = File('${backupDir.path}/$fileName');

      // In a real implementation, gather all data from database
      final backupData = {
        'version': '1.0.0',
        'timestamp': DateTime.now().toIso8601String(),
        'data': {
          'students': [],
          'classes': [],
          'attendance': [],
          'settings': {},
        }
      };

      // Write backup file
      await backupFile.writeAsString(
        const JsonEncoder.withIndent('  ').convert(backupData),
      );

      return backupFile;
    } catch (e) {
      throw Exception('Failed to create backup: $e');
    }
  }

  /// Restore data from a backup file
  Future<void> restoreBackup(File backupFile) async {
    try {
      if (!await backupFile.exists()) {
        throw Exception('Backup file does not exist');
      }

      final contents = await backupFile.readAsString();
      final backupData = jsonDecode(contents) as Map<String, dynamic>;

      // Validate backup structure
      if (!backupData.containsKey('version') ||
          !backupData.containsKey('data')) {
        throw Exception('Invalid backup file format');
      }

      // In a real implementation, restore data to database
      // This would involve:
      // 1. Clear existing data (with confirmation)
      // 2. Import students, classes, attendance records
      // 3. Restore settings
    } catch (e) {
      throw Exception('Failed to restore backup: $e');
    }
  }

  /// Export data to Excel format
  Future<File> exportToExcel() async {
    try {
      final timestamp = DateFormat('yyyyMMdd_HHmmss').format(DateTime.now());
      final fileName = 'attendease_export_$timestamp.xlsx';

      final directory = await getApplicationDocumentsDirectory();
      final exportDir = Directory('${directory.path}/exports');

      if (!await exportDir.exists()) {
        await exportDir.create(recursive: true);
      }

      final exportFile = File('${exportDir.path}/$fileName');

      // In a real implementation, use the excel package to create workbook
      // with sheets for students, classes, and attendance

      return exportFile;
    } catch (e) {
      throw Exception('Failed to export data: $e');
    }
  }

  /// Import data from Excel file
  Future<void> importFromExcel(File excelFile) async {
    try {
      if (!await excelFile.exists()) {
        throw Exception('Import file does not exist');
      }

      // In a real implementation, use the excel package to read workbook
      // and import data into database
    } catch (e) {
      throw Exception('Failed to import data: $e');
    }
  }

  /// Get list of available backups
  Future<List<File>> getBackupFiles() async {
    try {
      final directory = await getApplicationDocumentsDirectory();
      final backupDir = Directory('${directory.path}/backups');

      if (!await backupDir.exists()) {
        return [];
      }

      final files = await backupDir
          .list()
          .where((entity) => entity is File && entity.path.endsWith('.json'))
          .cast<File>()
          .toList();

      // Sort by modification date (newest first)
      files
          .sort((a, b) => b.lastModifiedSync().compareTo(a.lastModifiedSync()));

      return files;
    } catch (e) {
      return [];
    }
  }

  /// Delete a backup file
  Future<void> deleteBackup(File backupFile) async {
    try {
      if (await backupFile.exists()) {
        await backupFile.delete();
      }
    } catch (e) {
      throw Exception('Failed to delete backup: $e');
    }
  }
}
