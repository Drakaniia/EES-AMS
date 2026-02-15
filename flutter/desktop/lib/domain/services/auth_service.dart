import '../entities/user_entity.dart';
import '../repositories/user_repository.dart';
import '../core/result.dart';
import '../exceptions/domain_exceptions.dart';

/// Service for handling authentication-related business logic.
/// 
/// This service encapsulates the use cases related to user authentication,
/// coordinating between the presentation layer and the repository layer.
class AuthService {
  final UserRepository _userRepository;

  /// Creates a new [AuthService] with the provided [UserRepository].
  AuthService(this._userRepository);

  /// Signs in a user with email and password.
  /// 
  /// Validates input, authenticates the user, and returns the user entity.
  /// Returns failure result with [ValidationException] for invalid input,
  /// or [AuthenticationException] for authentication failures.
  Future<Result<UserEntity>> signIn({
    required String email,
    required String password,
  }) async {
    // Validate email
    if (email.trim().isEmpty) {
      return Result.failure(
        const ValidationException('Email is required'),
      );
    }

    final emailRegex = RegExp(r'^[^@]+@[^@]+\.[^@]+$');
    if (!emailRegex.hasMatch(email)) {
      return Result.failure(
        const ValidationException('Invalid email format'),
      );
    }

    // Validate password
    if (password.trim().isEmpty) {
      return Result.failure(
        const ValidationException('Password is required'),
      );
    }

    // Authenticate user
    return await _userRepository.authenticate(email, password);
  }

  /// Signs up a new user with the provided details.
  /// 
  /// Validates input, creates the user account, and returns the new user entity.
  /// Returns failure result with [ValidationException] for invalid input,
  /// or [DataException] for account creation failures.
  Future<Result<UserEntity>> signUp({
    required String email,
    required String displayName,
    required String password,
    String? schoolName,
    UserRole role = UserRole.student,
  }) async {
    // Validate email
    if (email.trim().isEmpty) {
      return Result.failure(
        const ValidationException('Email is required'),
      );
    }

    final emailRegex = RegExp(r'^[^@]+@[^@]+\.[^@]+$');
    if (!emailRegex.hasMatch(email)) {
      return Result.failure(
        const ValidationException('Invalid email format'),
      );
    }

    // Validate display name
    if (displayName.trim().isEmpty) {
      return Result.failure(
        const ValidationException('Display name is required'),
      );
    }

    if (displayName.trim().length < 2) {
      return Result.failure(
        const ValidationException('Display name must be at least 2 characters'),
      );
    }

    // Validate password
    if (password.trim().isEmpty) {
      return Result.failure(
        const ValidationException('Password is required'),
      );
    }

    if (password.length < 6) {
      return Result.failure(
        const ValidationException('Password must be at least 6 characters'),
      );
    }

    // Create user account
    return await _userRepository.register(
      email: email,
      displayName: displayName,
      password: password,
      schoolName: schoolName,
      role: role,
    );
  }

  /// Signs out the current user.
  /// 
  /// Returns success result on sign out.
  Future<Result<void>> signOut() async {
    return await _userRepository.signOut();
  }

  /// Gets the currently authenticated user.
  /// 
  /// Returns the user entity if authenticated, failure result otherwise.
  Future<Result<UserEntity>> getCurrentUser() async {
    return await _userRepository.getCurrentUser();
  }

  /// Checks if a user is currently authenticated.
  /// 
  /// Returns true if authenticated, false otherwise.
  Future<bool> isAuthenticated() async {
    return await _userRepository.isAuthenticated();
  }

  /// Changes the user's password.
  /// 
  /// Validates input and updates the password.
  /// Returns failure result with [ValidationException] for invalid input,
  /// or [AuthenticationException] if current password is incorrect.
  Future<Result<void>> changePassword({
    required String currentPassword,
    required String newPassword,
  }) async {
    // Validate current password
    if (currentPassword.trim().isEmpty) {
      return Result.failure(
        const ValidationException('Current password is required'),
      );
    }

    // Validate new password
    if (newPassword.trim().isEmpty) {
      return Result.failure(
        const ValidationException('New password is required'),
      );
    }

    if (newPassword.length < 6) {
      return Result.failure(
        const ValidationException('New password must be at least 6 characters'),
      );
    }

    if (currentPassword == newPassword) {
      return Result.failure(
        const ValidationException('New password must be different from current password'),
      );
    }

    // Change password
    return await _userRepository.changePassword(
      currentPassword: currentPassword,
      newPassword: newPassword,
    );
  }

  /// Updates the user's profile information.
  /// 
  /// Validates input and updates the user profile.
  /// Returns failure result with [ValidationException] for invalid input.
  Future<Result<UserEntity>> updateProfile({
    required UserEntity user,
    String? displayName,
    String? schoolName,
  }) async {
    // Validate display name if provided
    if (displayName != null && displayName.trim().isEmpty) {
      return Result.failure(
        const ValidationException('Display name cannot be empty'),
      );
    }

    if (displayName != null && displayName.trim().length < 2) {
      return Result.failure(
        const ValidationException('Display name must be at least 2 characters'),
      );
    }

    // Update user with new values
    final updatedUser = user.copyWith(
      displayName: displayName ?? user.displayName,
      schoolName: schoolName ?? user.schoolName,
    );

    return await _userRepository.updateUser(updatedUser);
  }
}