import 'package:shared_preferences/shared_preferences.dart';

/// Service for managing application configuration
class ConfigService {
  static const String _keyBaseUrl = 'base_url';
  static const String _keyDefaultBaseUrl = 'http://localhost:8080/api';

  /// Get the configured base URL for API calls
  Future<String> getBaseUrl() async {
    final prefs = await SharedPreferences.getInstance();
    return prefs.getString(_keyBaseUrl) ?? _keyDefaultBaseUrl;
  }

  /// Set the base URL for API calls
  Future<void> setBaseUrl(String url) async {
    final prefs = await SharedPreferences.getInstance();
    await prefs.setString(_keyBaseUrl, url);
  }

  /// Reset base URL to default
  Future<void> resetBaseUrl() async {
    final prefs = await SharedPreferences.getInstance();
    await prefs.remove(_keyBaseUrl);
  }
}
