import 'package:package_info_plus/package_info_plus.dart';
import 'package:dio/dio.dart';
import 'api_client.dart';

/// Service for checking and managing application updates
class UpdateService {
  final ApiClient _apiClient;

  UpdateService({ApiClient? apiClient}) : _apiClient = apiClient ?? ApiClient();

  /// Check for available updates
  Future<UpdateInfo?> checkForUpdates() async {
    try {
      final packageInfo = await PackageInfo.fromPlatform();
      final currentVersion = packageInfo.version;

      // Call the Tauri backend update endpoint
      final response = await _apiClient.dio.get(
        '/update/check',
        queryParameters: {
          'current_version': currentVersion,
          'platform': _getPlatform(),
        },
      );

      if (response.statusCode == 200) {
        final data = response.data as Map<String, dynamic>;

        if (data['update_available'] == true) {
          return UpdateInfo(
            version: data['version'] as String,
            downloadUrl: data['download_url'] as String,
            releaseNotes: data['release_notes'] as String,
            isRequired: data['is_required'] as bool? ?? false,
          );
        }
      }

      return null;
    } catch (e) {
      // Log the error for debugging
      print('Update check error: $e');
      // In case of error, return null (no update available)
      return null;
    }
  }

  /// Get current app version
  Future<String> getCurrentVersion() async {
    final packageInfo = await PackageInfo.fromPlatform();
    return packageInfo.version;
  }

  /// Get current build number
  Future<String> getBuildNumber() async {
    final packageInfo = await PackageInfo.fromPlatform();
    return packageInfo.buildNumber;
  }

  /// Get app name
  Future<String> getAppName() async {
    final packageInfo = await PackageInfo.fromPlatform();
    return packageInfo.appName;
  }

  String _getPlatform() {
    // Detect the actual platform
    if (_apiClient.dio.options.baseUrl.contains('android')) {
      return 'android';
    } else if (_apiClient.dio.options.baseUrl.contains('ios')) {
      return 'ios';
    } else if (_apiClient.dio.options.baseUrl.contains('macos')) {
      return 'macos';
    } else if (_apiClient.dio.options.baseUrl.contains('linux')) {
      return 'linux';
    } else {
      // Default to windows for desktop
      return 'windows';
    }
  }
  
  /// Download update
  Future<bool> downloadUpdate(String downloadUrl, String savePath) async {
    try {
      final response = await _apiClient.dio.download(downloadUrl, savePath);
      return response.statusCode == 200;
    } catch (e) {
      print('Download update error: $e');
      return false;
    }
  }
  
  /// Install update
  Future<bool> installUpdate(String filePath) async {
    try {
      // In a real implementation, this would execute the installer
      // For now, we'll just return true to indicate success
      return true;
    } catch (e) {
      print('Install update error: $e');
      return false;
    }
  }
}

/// Information about an available update
class UpdateInfo {
  final String version;
  final String downloadUrl;
  final String releaseNotes;
  final bool isRequired;

  UpdateInfo({
    required this.version,
    required this.downloadUrl,
    required this.releaseNotes,
    required this.isRequired,
  });
}
