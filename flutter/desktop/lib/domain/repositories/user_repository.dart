import '../entities/user_entity.dart';
import '../core/result.dart';
import '../exceptions/domain_exceptions.dart';

/// Repository interface for user-related data operations.
/// 
/// This interface defines the contract for user data access without
/// specifying the implementation details, following the Repository pattern.
abstract class UserRepository {
  /// Authenticates a user with the provided credentials.
  /// 
  /// Returns [UserEntity] on success, throws [AuthenticationException] on failure.
  Future<Result<UserEntity>> authenticate(String email, String password);

  /// Registers a new user in the system.
  /// 
  /// Returns [UserEntity] on success, throws [ValidationException] on invalid data
  /// or [DataException] on registration failure.
  Future<Result<UserEntity>> register({
    required String email,
    required String displayName,
    required String password,
    String? schoolName,
    UserRole role = UserRole.student,
  });

  /// Retrieves the current authenticated user.
  /// 
  /// Returns [UserEntity] if a user is authenticated, returns failure result
  /// with [AuthenticationException] if no user is authenticated.
  Future<Result<UserEntity>> getCurrentUser();

  /// Updates user information.
  /// 
  /// Returns updated [UserEntity] on success, throws [ValidationException]
  /// on invalid data or [DataException] on update failure.
  Future<Result<UserEntity>> updateUser(UserEntity user);

  /// Changes the user's password.
  /// 
  /// Returns success result on password change, throws [AuthenticationException]
  /// if current password is incorrect or [ValidationException] if new password is invalid.
  Future<Result<void>> changePassword({
    required String currentPassword,
    required String newPassword,
  });

  /// Signs out the current user.
  /// 
  /// Returns success result on sign out.
  Future<Result<void>> signOut();

  /// Refreshes the authentication token.
  /// 
  /// Returns new token on success, throws [AuthenticationException] on failure.
  Future<Result<String>> refreshToken();

  /// Checks if a user is currently authenticated.
  /// 
  /// Returns true if authenticated, false otherwise.
  Future<bool> isAuthenticated();
}