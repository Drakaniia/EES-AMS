/// Base exception class for all domain errors.
abstract class DomainException implements Exception {
  /// The error message.
  final String message;
  
  /// Optional error code.
  final String? code;
  
  /// Creates a new [DomainException].
  const DomainException(this.message, {this.code});
  
  @override
  String toString() => 'DomainException: $message${code != null ? ' (Code: $code)' : ''}';
}

/// Exception for validation errors.
class ValidationException extends DomainException {
  /// Creates a new [ValidationException].
  const ValidationException(super.message, {super.code});
  
  @override
  String toString() => 'ValidationException: $message${code != null ? ' (Code: $code)' : ''}';
}

/// Exception for network-related errors.
class NetworkException extends DomainException {
  /// Creates a new [NetworkException].
  const NetworkException(super.message, {super.code});
  
  @override
  String toString() => 'NetworkException: $message${code != null ? ' (Code: $code)' : ''}';
}

/// Exception for authentication errors.
class AuthenticationException extends DomainException {
  /// Creates a new [AuthenticationException].
  const AuthenticationException(super.message, {super.code});
  
  @override
  String toString() => 'AuthenticationException: $message${code != null ? ' (Code: $code)' : ''}';
}

/// Exception for data access errors.
class DataException extends DomainException {
  /// Creates a new [DataException].
  const DataException(super.message, {super.code});
  
  @override
  String toString() => 'DataException: $message${code != null ? ' (Code: $code)' : ''}';
}

/// Exception for authorization errors.
class AuthorizationException extends DomainException {
  /// Creates a new [AuthorizationException].
  const AuthorizationException(super.message, {super.code});
  
  @override
  String toString() => 'AuthorizationException: $message${code != null ? ' (Code: $code)' : ''}';
}