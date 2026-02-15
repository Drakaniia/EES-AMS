import 'package:flutter_riverpod/flutter_riverpod.dart';
import '../../services/auth_service.dart';
import '../../services/secure_storage_service.dart';
import '../../models/user.dart';
import 'student_providers.dart';

// Provider for SecureStorageService
final secureStorageProvider = Provider<SecureStorageService>((ref) {
  return SecureStorageService();
});

// Provider for AuthService
final authServiceProvider = Provider<AuthService>((ref) {
  final dio = ref.watch(apiClientProvider);
  return AuthService(dio);
});

// Provider for authentication state
final authStateProvider = StateNotifierProvider<AuthNotifier, AuthState>((ref) {
  final service = ref.watch(authServiceProvider);
  final storage = ref.watch(secureStorageProvider);
  return AuthNotifier(service, storage);
});

// Authentication state
class AuthState {
  final bool isAuthenticated;
  final bool isLoading;
  final String? error;
  final User? user;

  AuthState({
    required this.isAuthenticated,
    required this.isLoading,
    this.error,
    this.user,
  });

  AuthState copyWith({
    bool? isAuthenticated,
    bool? isLoading,
    String? error,
    User? user,
  }) {
    return AuthState(
      isAuthenticated: isAuthenticated ?? this.isAuthenticated,
      isLoading: isLoading ?? this.isLoading,
      error: error ?? this.error,
      user: user ?? this.user,
    );
  }
}

// Authentication notifier
class AuthNotifier extends StateNotifier<AuthState> {
  final AuthService _authService;
  final SecureStorageService _secureStorage;

  AuthNotifier(this._authService, this._secureStorage)
      : super(AuthState(isAuthenticated: false, isLoading: false)) {
    // Check auth status on initialization
    checkAuthStatus();
  }

  Future<void> login(String email, String password) async {
    state = state.copyWith(isLoading: true, error: null);

    try {
      final credentials = AuthCredentials(email: email, password: password);
      final response = await _authService.login(credentials);

      if (response.success && response.data != null) {
        // Store token securely
        await _secureStorage.storeAuthTokens(
          accessToken: response.data!.token,
          refreshToken: response.data!.refreshToken,
        );

        await _secureStorage.storeUserInfo(
          userId: response.data!.user.id.toString(),
          email: response.data!.user.email,
        );

        state = state.copyWith(
          isAuthenticated: true,
          isLoading: false,
          user: response.data!.user,
        );
      } else {
        state = state.copyWith(
          isAuthenticated: false,
          isLoading: false,
          error: response.message ?? 'Login failed',
        );
      }
    } catch (e) {
      state = state.copyWith(
        isAuthenticated: false,
        isLoading: false,
        error: 'Network error: ${e.toString()}',
      );
    }
  }

  Future<void> logout() async {
    state = state.copyWith(isLoading: true);

    try {
      final token = await _secureStorage.getAccessToken();
      if (token != null) {
        try {
          await _authService.logout('Bearer $token');
        } catch (e) {
          // Ignore logout API errors, still clear local data
        }
      }

      // Clear stored token and user data
      await _secureStorage.clearAuthData();

      state = AuthState(isAuthenticated: false, isLoading: false);
    } catch (e) {
      state = state.copyWith(
          isLoading: false, error: 'Logout error: ${e.toString()}');
    }
  }

  Future<void> checkAuthStatus() async {
    state = state.copyWith(isLoading: true);

    try {
      // Check if valid token exists
      final isAuth = await _secureStorage.isAuthenticated();

      if (isAuth) {
        final token = await _secureStorage.getAccessToken();
        if (token != null) {
          try {
            // Verify token by fetching current user
            final response = await _authService.getCurrentUser('Bearer $token');

            if (response.success && response.data != null) {
              state = state.copyWith(
                isAuthenticated: true,
                isLoading: false,
                user: response.data,
              );
              return;
            }
          } catch (e) {
            // Token is invalid, clear it
            await _secureStorage.clearAuthData();
          }
        }
      }

      state = state.copyWith(isAuthenticated: false, isLoading: false);
    } catch (e) {
      state = state.copyWith(
          isLoading: false, error: 'Auth check error: ${e.toString()}');
    }
  }

  Future<void> updateProfile(Map<String, dynamic> profileData) async {
    state = state.copyWith(isLoading: true, error: null);

    try {
      final token = await _secureStorage.getAccessToken();
      if (token == null) {
        throw Exception('Not authenticated');
      }

      final response = await _authService.updateProfile(
        'Bearer $token',
        profileData,
      );

      if (response.success && response.data != null) {
        state = state.copyWith(
          isLoading: false,
          user: response.data,
        );
      } else {
        state = state.copyWith(
          isLoading: false,
          error: response.message ?? 'Update failed',
        );
      }
    } catch (e) {
      state = state.copyWith(
        isLoading: false,
        error: 'Update error: ${e.toString()}',
      );
    }
  }

  Future<void> changePassword(Map<String, dynamic> passwordData) async {
    state = state.copyWith(isLoading: true, error: null);

    try {
      final token = await _secureStorage.getAccessToken();
      if (token == null) {
        throw Exception('Not authenticated');
      }

      final response = await _authService.changePassword(
        'Bearer $token',
        passwordData,
      );

      if (response.success) {
        state = state.copyWith(isLoading: false);
      } else {
        state = state.copyWith(
          isLoading: false,
          error: response.message ?? 'Password change failed',
        );
      }
    } catch (e) {
      state = state.copyWith(
        isLoading: false,
        error: 'Password change error: ${e.toString()}',
      );
    }
  }
}
