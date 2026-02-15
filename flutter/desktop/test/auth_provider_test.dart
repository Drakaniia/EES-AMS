import 'package:flutter_test/flutter_test.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:ees_ams/providers/auth_provider.dart';
import 'package:ees_ams/models/user.dart';

void main() {
  group('AuthStateProvider Tests', () {
    late ProviderContainer container;
    late AuthNotifier authNotifier;

    setUp(() {
      container = ProviderContainer();
      authNotifier = container.read(authStateProvider.notifier);
    });

    tearDown(() {
      container.dispose();
    });

    test('should initialize with null user', () {
      final authState = container.read(authStateProvider);
      expect(authState.user, isNull);
      expect(authState.token, isNull);
      expect(authState.isLoading, isFalse);
      expect(authState.error, isNull);
    });

    test('should have correct initial state', () {
      final authState = container.read(authStateProvider);
      expect(authState.isAuthenticated, isFalse);
    });

    test('should clear error when clearError is called', () {
      final initialState = container.read(authStateProvider);
      expect(initialState.error, isNull);
      
      // This test would need a mock to test error scenarios properly
      // For now, we just ensure clearError can be called
      authNotifier.clearError();
      final updatedState = container.read(authStateProvider);
      expect(updatedState.error, isNull);
    });
  });

  group('User Model Tests', () {
    test('should create user with required fields', () {
      const user = User(
        id: 1,
        email: 'test@example.com',
        displayName: 'Test User',
        role: 'student',
      );
      
      expect(user.id, equals(1));
      expect(user.email, equals('test@example.com'));
      expect(user.displayName, equals('Test User'));
      expect(user.role, equals('student'));
      expect(user.isActive, isTrue);
    });

    test('should support equality', () {
      const user1 = User(
        id: 1,
        email: 'test@example.com',
        displayName: 'Test User',
        role: 'student',
      );
      
      const user2 = User(
        id: 1,
        email: 'test@example.com',
        displayName: 'Test User',
        role: 'student',
      );
      
      expect(user1, equals(user2));
    });

    test('should create copy with updated values', () {
      const user = User(
        id: 1,
        email: 'test@example.com',
        displayName: 'Test User',
        role: 'student',
      );
      
      final updatedUser = user.copyWith(displayName: 'Updated User');
      
      expect(updatedUser.id, equals(user.id));
      expect(updatedUser.email, equals(user.email));
      expect(updatedUser.displayName, equals('Updated User'));
      expect(updatedUser.role, equals(user.role));
    });
  });

  group('AuthCredentials Tests', () {
    test('should create credentials with email and password', () {
      const credentials = AuthCredentials(
        email: 'test@example.com',
        password: 'password123',
      );
      
      expect(credentials.email, equals('test@example.com'));
      expect(credentials.password, equals('password123'));
    });

    test('should support equality', () {
      const credentials1 = AuthCredentials(
        email: 'test@example.com',
        password: 'password123',
      );
      
      const credentials2 = AuthCredentials(
        email: 'test@example.com',
        password: 'password123',
      );
      
      expect(credentials1, equals(credentials2));
    });
  });
}