import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:shared_preferences/shared_preferences.dart';
import 'package:logger/logger.dart';
import '../models/user.dart';
import '../services/auth_service.dart';
import '../services/api_client.dart';

// Auth SharedPreferences provider
final authSharedPreferencesProvider = Provider<SharedPreferences>((ref) {
  throw UnimplementedError(
      'authSharedPreferencesProvider must be overridden in main.dart');
});

// Logger provider
final loggerProvider = Provider<Logger>((ref) {
  return Logger();
});

// Auth state
class AuthState {
  final User? user;
  final String? token;
  final bool isLoading;
  final String? error;

  const AuthState({
    this.user,
    this.token,
    this.isLoading = false,
    this.error,
  });

  AuthState copyWith({
    User? user,
    String? token,
    bool? isLoading,
    String? error,
  }) {
    return AuthState(
      user: user ?? this.user,
      token: token ?? this.token,
      isLoading: isLoading ?? this.isLoading,
      error: error ?? this.error,
    );
  }

  bool get isAuthenticated => user != null && token != null;
}

// Auth state provider
final authStateProvider = StateNotifierProvider<AuthNotifier, AuthState>((ref) {
  final authService = ref.watch(authServiceProvider);
  final sharedPreferences = ref.watch(authSharedPreferencesProvider);
  return AuthNotifier(authService, sharedPreferences);
});

class AuthNotifier extends StateNotifier<AuthState> {
  final AuthService _authService;
  final SharedPreferences _sharedPreferences;
  final Logger _logger;

  AuthNotifier(this._authService, this._sharedPreferences, [Logger? logger])
      : _logger = logger ?? Logger(),
        super(const AuthState()) {
    _initializeAuth();
  }

  Future<void> _initializeAuth() async {
    try {
      final token = _sharedPreferences.getString('auth_token');
      if (token != null) {
        final response = await _authService.getCurrentUser('Bearer $token');
        if (response.isSuccess && response.data != null) {
          state = state.copyWith(
            user: response.data!,
            token: token,
            isLoading: false,
          );
        } else {
          await _sharedPreferences.remove('auth_token');
          state = const AuthState();
        }
      }
    } catch (e) {
      _logger.e('Error initializing auth: $e');
      state = state.copyWith(isLoading: false);
    }
  }

  Future<void> login(String email, String password) async {
    state = state.copyWith(isLoading: true, error: null);

    try {
      final credentials = AuthCredentials(email: email, password: password);
      final response = await _authService.login(credentials);

      if (response.isSuccess && response.data != null) {
        final authResponse = response.data!;
        await _sharedPreferences.setString('auth_token', authResponse.token);
        await _sharedPreferences.setString(
            'refresh_token', authResponse.refreshToken);

        state = state.copyWith(
          user: authResponse.user,
          token: authResponse.token,
          isLoading: false,
          error: null,
        );
      } else {
        state = state.copyWith(
          isLoading: false,
          error: response.error ?? response.message ?? 'Login failed',
        );
      }
    } catch (e) {
      _logger.e('Login error: $e');
      state = state.copyWith(
        isLoading: false,
        error: 'An unexpected error occurred during login',
      );
    }
  }

  Future<void> register(Map<String, dynamic> userData) async {
    state = state.copyWith(isLoading: true, error: null);

    try {
      final response = await _authService.register(userData);

      if (response.isSuccess && response.data != null) {
        final authResponse = response.data!;
        await _sharedPreferences.setString('auth_token', authResponse.token);
        await _sharedPreferences.setString(
            'refresh_token', authResponse.refreshToken);

        state = state.copyWith(
          user: authResponse.user,
          token: authResponse.token,
          isLoading: false,
          error: null,
        );
      } else {
        state = state.copyWith(
          isLoading: false,
          error: response.error ?? response.message ?? 'Registration failed',
        );
      }
    } catch (e) {
      _logger.e('Registration error: $e');
      state = state.copyWith(
        isLoading: false,
        error: 'An unexpected error occurred during registration',
      );
    }
  }

  Future<void> signInWithGoogle(Map<String, dynamic> googleData) async {
    state = state.copyWith(isLoading: true, error: null);

    try {
      final response = await _authService.signInWithGoogle(googleData);

      if (response.isSuccess && response.data != null) {
        final authResponse = response.data!;
        await _sharedPreferences.setString('auth_token', authResponse.token);
        await _sharedPreferences.setString(
            'refresh_token', authResponse.refreshToken);

        state = state.copyWith(
          user: authResponse.user,
          token: authResponse.token,
          isLoading: false,
          error: null,
        );
      } else {
        state = state.copyWith(
          isLoading: false,
          error: response.error ?? response.message ?? 'Google sign in failed',
        );
      }
    } catch (e) {
      _logger.e('Google sign in error: $e');
      state = state.copyWith(
        isLoading: false,
        error: 'An unexpected error occurred during Google sign in',
      );
    }
  }

  Future<void> logout() async {
    try {
      final token = state.token;
      if (token != null) {
        await _authService.logout('Bearer $token');
      }
    } catch (e) {
      _logger.e('Logout error: $e');
    } finally {
      await _sharedPreferences.remove('auth_token');
      await _sharedPreferences.remove('refresh_token');
      state = const AuthState();
    }
  }

  Future<void> updateProfile(Map<String, dynamic> profileData) async {
    final token = state.token;
    if (token == null) {
      state = state.copyWith(error: 'Not authenticated');
      return;
    }

    state = state.copyWith(isLoading: true, error: null);

    try {
      final response =
          await _authService.updateProfile('Bearer $token', profileData);

      if (response.isSuccess && response.data != null) {
        state = state.copyWith(
          user: response.data!,
          isLoading: false,
          error: null,
        );
      } else {
        state = state.copyWith(
          isLoading: false,
          error: response.error ?? response.message ?? 'Profile update failed',
        );
      }
    } catch (e) {
      _logger.e('Profile update error: $e');
      state = state.copyWith(
        isLoading: false,
        error: 'An unexpected error occurred during profile update',
      );
    }
  }

  Future<void> changePassword(Map<String, dynamic> passwordData) async {
    final token = state.token;
    if (token == null) {
      state = state.copyWith(error: 'Not authenticated');
      return;
    }

    state = state.copyWith(isLoading: true, error: null);

    try {
      final response =
          await _authService.changePassword('Bearer $token', passwordData);

      if (response.isSuccess) {
        state = state.copyWith(
          isLoading: false,
          error: null,
        );
      } else {
        state = state.copyWith(
          isLoading: false,
          error: response.error ?? response.message ?? 'Password change failed',
        );
      }
    } catch (e) {
      _logger.e('Change password error: $e');
      state = state.copyWith(
        isLoading: false,
        error: 'An unexpected error occurred during password change',
      );
    }
  }

  void clearError() {
    state = state.copyWith(error: null);
  }
}

// Provider to check if user is authenticated
final isAuthenticatedProvider = Provider<bool>((ref) {
  final authState = ref.watch(authStateProvider);
  return authState.token != null && authState.user != null;
});

// Provider for current user
final currentUserProvider = Provider<User?>((ref) {
  final authState = ref.watch(authStateProvider);
  return authState.user;
});
