import 'package:shared_preferences/shared_preferences.dart';

/// Service for storing data like auth tokens
/// Note: Using shared_preferences instead of flutter_secure_storage for compatibility
/// In production, you should use flutter_secure_storage for better security
class SecureStorageService {
  // Keys
  static const String _keyAccessToken = 'access_token';
  static const String _keyRefreshToken = 'refresh_token';
  static const String _keyUserId = 'user_id';
  static const String _keyUserEmail = 'user_email';

  Future<SharedPreferences> _getPrefs() async {
    return await SharedPreferences.getInstance();
  }

  /// Store authentication tokens
  Future<void> storeAuthTokens({
    required String accessToken,
    String? refreshToken,
  }) async {
    final prefs = await _getPrefs();
    await prefs.setString(_keyAccessToken, accessToken);
    if (refreshToken != null) {
      await prefs.setString(_keyRefreshToken, refreshToken);
    }
  }

  /// Get access token
  Future<String?> getAccessToken() async {
    final prefs = await _getPrefs();
    return prefs.getString(_keyAccessToken);
  }

  /// Get refresh token
  Future<String?> getRefreshToken() async {
    final prefs = await _getPrefs();
    return prefs.getString(_keyRefreshToken);
  }

  /// Store user information
  Future<void> storeUserInfo({
    required String userId,
    required String email,
  }) async {
    final prefs = await _getPrefs();
    await prefs.setString(_keyUserId, userId);
    await prefs.setString(_keyUserEmail, email);
  }

  /// Get user ID
  Future<String?> getUserId() async {
    final prefs = await _getPrefs();
    return prefs.getString(_keyUserId);
  }

  /// Get user email
  Future<String?> getUserEmail() async {
    final prefs = await _getPrefs();
    return prefs.getString(_keyUserEmail);
  }

  /// Check if user is authenticated (has valid token)
  Future<bool> isAuthenticated() async {
    final token = await getAccessToken();
    return token != null && token.isNotEmpty;
  }

  /// Clear all stored authentication data
  Future<void> clearAuthData() async {
    final prefs = await _getPrefs();
    await prefs.remove(_keyAccessToken);
    await prefs.remove(_keyRefreshToken);
    await prefs.remove(_keyUserId);
    await prefs.remove(_keyUserEmail);
  }

  /// Clear all stored data
  Future<void> clearAll() async {
    final prefs = await _getPrefs();
    await prefs.clear();
  }
}
