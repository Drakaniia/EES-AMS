import 'package:connectivity_plus/connectivity_plus.dart';
import 'package:shared_preferences/shared_preferences.dart';
import 'package:dio/dio.dart';
import 'api_client.dart';
import 'attendance_service.dart';
import 'class_service.dart';
import 'student_service.dart';

/// Service for managing data synchronization
class SyncService {
  static const String _keyLastSync = 'last_sync_timestamp';
  final ApiClient _apiClient;

  SyncService({ApiClient? apiClient}) : _apiClient = apiClient ?? ApiClient();

  /// Check network connectivity
  Future<bool> isConnected() async {
    var connectivityResult = await Connectivity().checkConnectivity();
    return connectivityResult != ConnectivityResult.none;
  }

  /// Perform manual sync with backend
  Future<bool> syncNow(String token) async {
    try {
      // Check network connectivity
      if (!await isConnected()) {
        throw Exception('No internet connection available for sync');
      }

      // Set authorization token
      _apiClient.setAuthToken(token);

      // Perform sync operations
      await _syncClasses(token);
      await _syncStudents(token);
      await _syncAttendance(token);

      // Update last sync timestamp
      await _updateLastSyncTime();

      return true;
    } catch (e) {
      throw Exception('Sync failed: $e');
    }
  }

  /// Sync classes with backend
  Future<void> _syncClasses(String token) async {
    try {
      // In a real implementation, you would:
      // 1. Get locally modified classes that need to be synced
      // 2. Upload them to the server
      // 3. Download any new/updated classes from server
      // 4. Merge and update local database
    } catch (e) {
      throw Exception('Failed to sync classes: $e');
    }
  }

  /// Sync students with backend
  Future<void> _syncStudents(String token) async {
    try {
      // In a real implementation, you would:
      // 1. Get locally modified students that need to be synced
      // 2. Upload them to the server
      // 3. Download any new/updated students from server
      // 4. Merge and update local database
    } catch (e) {
      throw Exception('Failed to sync students: $e');
    }
  }

  /// Sync attendance records with backend
  Future<void> _syncAttendance(String token) async {
    try {
      // In a real implementation, you would:
      // 1. Get locally modified attendance records that need to be synced
      // 2. Upload them to the server
      // 3. Download any new/updated attendance records from server
      // 4. Merge and update local database
    } catch (e) {
      throw Exception('Failed to sync attendance: $e');
    }
  }

  /// Get last sync timestamp
  Future<DateTime?> getLastSyncTime() async {
    try {
      final prefs = await SharedPreferences.getInstance();
      final timestamp = prefs.getInt(_keyLastSync);

      if (timestamp != null) {
        return DateTime.fromMillisecondsSinceEpoch(timestamp);
      }
      return null;
    } catch (e) {
      return null;
    }
  }

  /// Update last sync timestamp
  Future<void> _updateLastSyncTime() async {
    final prefs = await SharedPreferences.getInstance();
    await prefs.setInt(_keyLastSync, DateTime.now().millisecondsSinceEpoch);
  }

  /// Format last sync time for display
  String formatLastSyncTime(DateTime? lastSync) {
    if (lastSync == null) {
      return 'Never';
    }

    final now = DateTime.now();
    final difference = now.difference(lastSync);

    if (difference.inSeconds < 60) {
      return 'Just now';
    } else if (difference.inMinutes < 60) {
      return '${difference.inMinutes} minute${difference.inMinutes > 1 ? 's' : ''} ago';
    } else if (difference.inHours < 24) {
      return '${difference.inHours} hour${difference.inHours > 1 ? 's' : ''} ago';
    } else if (difference.inDays < 7) {
      return '${difference.inDays} day${difference.inDays > 1 ? 's' : ''} ago';
    } else {
      return '${lastSync.day}/${lastSync.month}/${lastSync.year}';
    }
  }

  /// Check if sync is needed based on interval
  Future<bool> shouldSync(int intervalMinutes) async {
    final lastSync = await getLastSyncTime();

    if (lastSync == null) {
      return true;
    }

    final now = DateTime.now();
    final difference = now.difference(lastSync);

    return difference.inMinutes >= intervalMinutes;
  }
}
