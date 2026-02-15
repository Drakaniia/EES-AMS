import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:shared_preferences/shared_preferences.dart';

// Shared preferences provider
final sharedPreferencesProvider = Provider<SharedPreferences>((ref) {
  throw UnimplementedError(
      'SharedPreferences must be overridden in provider scope');
});

// Dark mode provider
final darkModeProvider = StateProvider<bool>((ref) {
  return false; // Default to light mode
});

// Initialize SharedPreferences
Future<SharedPreferences> getSharedPreferences() async {
  return await SharedPreferences.getInstance();
}

// Initialize dark mode from SharedPreferences
bool getDarkModePreference(SharedPreferences prefs) {
  return prefs.getBool('dark_mode') ?? false;
}

// Save dark mode preference
Future<void> saveDarkModePreference(
    SharedPreferences prefs, bool isDark) async {
  await prefs.setBool('dark_mode', isDark);
}
