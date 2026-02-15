import 'dart:io';
import 'package:path_provider/path_provider.dart';
import 'package:shared_preferences/shared_preferences.dart';

/// Service for managing application cache
class CacheService {
  /// Clear all application cache
  Future<void> clearCache() async {
    try {
      // Clear SharedPreferences cache (keep user settings)
      final prefs = await SharedPreferences.getInstance();
      final keysToKeep = [
        'auto_sync',
        'notifications',
        'auto_backup',
        'sync_interval',
        'auto_update',
        'dark_mode',
        'base_url',
      ];

      final allKeys = prefs.getKeys();
      for (final key in allKeys) {
        if (!keysToKeep.contains(key)) {
          await prefs.remove(key);
        }
      }

      // Clear temporary directory
      final tempDir = await getTemporaryDirectory();
      if (await tempDir.exists()) {
        await _deleteDirectory(tempDir);
      }

      // Clear application cache directory
      final cacheDir = await getApplicationCacheDirectory();
      if (await cacheDir.exists()) {
        await _deleteDirectory(cacheDir);
      }
    } catch (e) {
      throw Exception('Failed to clear cache: $e');
    }
  }

  /// Get cache size in bytes
  Future<int> getCacheSize() async {
    int totalSize = 0;

    try {
      final tempDir = await getTemporaryDirectory();
      if (await tempDir.exists()) {
        totalSize += await _getDirectorySize(tempDir);
      }

      final cacheDir = await getApplicationCacheDirectory();
      if (await cacheDir.exists()) {
        totalSize += await _getDirectorySize(cacheDir);
      }
    } catch (e) {
      // Ignore errors
    }

    return totalSize;
  }

  /// Format cache size for display
  String formatCacheSize(int bytes) {
    if (bytes < 1024) {
      return '$bytes B';
    } else if (bytes < 1024 * 1024) {
      return '${(bytes / 1024).toStringAsFixed(2)} KB';
    } else if (bytes < 1024 * 1024 * 1024) {
      return '${(bytes / (1024 * 1024)).toStringAsFixed(2)} MB';
    } else {
      return '${(bytes / (1024 * 1024 * 1024)).toStringAsFixed(2)} GB';
    }
  }

  Future<void> _deleteDirectory(Directory dir) async {
    if (await dir.exists()) {
      await dir.delete(recursive: true);
      await dir.create(); // Recreate empty directory
    }
  }

  Future<int> _getDirectorySize(Directory dir) async {
    int size = 0;
    try {
      if (await dir.exists()) {
        await for (final entity
            in dir.list(recursive: true, followLinks: false)) {
          if (entity is File) {
            size += await entity.length();
          }
        }
      }
    } catch (e) {
      // Ignore errors
    }
    return size;
  }
}
