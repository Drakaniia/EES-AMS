/// A wrapper for operations that may succeed or fail.
/// 
/// This is a common pattern in functional programming to handle errors
/// more explicitly than using exceptions everywhere.
class Result<T> {
  /// The success value, if the operation succeeded.
  final T? data;
  
  /// The error, if the operation failed.
  dynamic error;
  
  /// Whether the operation was successful.
  final bool isSuccess;
  
  /// Creates a successful result.
  Result.success(T value) : data = value, error = null, isSuccess = true;
  
  /// Creates a failed result.
  Result.failure(this.error) : data = null, isSuccess = false;
  
  /// Returns true if the result is successful.
  bool get isFailure => !isSuccess;
  
  /// Returns the data if successful, throws the error if failed.
  T get dataOrThrow {
    if (isSuccess) {
      return data as T;
    }
    throw error;
  }
  
  /// Maps the success value to a new value.
  Result<U> map<U>(U Function(T data) mapper) {
    if (isSuccess) {
      try {
        return Result.success(mapper(data as T));
      } catch (e) {
        return Result.failure(e);
      }
    }
    return Result.failure(error);
  }
  
  /// Maps the error to a new error.
  Result<T> mapError(dynamic Function(dynamic error) mapper) {
    if (isFailure) {
      try {
        return Result.failure(mapper(error));
      } catch (e) {
        return Result.failure(e);
      }
    }
    return Result.success(data as T);
  }
  
  /// Executes the function if the result is successful.
  Result<T> whenSuccess(void Function(T data) action) {
    if (isSuccess) {
      action(data as T);
    }
    return this;
  }
  
  /// Executes the function if the result failed.
  Result<T> whenFailure(void Function(dynamic error) action) {
    if (isFailure) {
      action(error);
    }
    return this;
  }
  
  /// Executes one of the functions based on the result.
  U fold<U>(U Function(T data) onSuccess, U Function(dynamic error) onFailure) {
    if (isSuccess) {
      return onSuccess(data as T);
    }
    return onFailure(error);
  }
  
  @override
  String toString() => isSuccess ? 'Success($data)' : 'Failure($error)';
  
  @override
  bool operator ==(Object other) {
    if (identical(this, other)) return true;
    return other is Result<T> &&
        other.data == data &&
        other.error == error &&
        other.isSuccess == isSuccess;
  }
  
  @override
  int get hashCode => data.hashCode ^ error.hashCode ^ isSuccess.hashCode;
}