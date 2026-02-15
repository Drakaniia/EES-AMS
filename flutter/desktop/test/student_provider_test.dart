import 'package:flutter_test/flutter_test.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:ees_ams/providers/student_provider.dart';
import 'package:ees_ams/models/student.dart';

void main() {
  group('StudentProvider Tests', () {
    late ProviderContainer container;
    late StudentNotifier studentNotifier;

    setUp(() {
      container = ProviderContainer();
      studentNotifier = container.read(studentProvider.notifier);
    });

    tearDown(() {
      container.dispose();
    });

    test('should initialize with empty student list', () {
      final studentState = container.read(studentProvider);
      expect(studentState.students, isEmpty);
      expect(studentState.selectedStudent, isNull);
      expect(studentState.isLoading, isFalse);
      expect(studentState.error, isNull);
    });

    test('should select student when selectStudent is called', () {
      const student = Student(
        id: 1,
        studentId: 'STU001',
        firstName: 'John',
        lastName: 'Doe',
        email: 'john@example.com',
      );
      
      studentNotifier.selectStudent(student);
      final studentState = container.read(studentProvider);
      expect(studentState.selectedStudent, equals(student));
    });

    test('should clear selected student when selectStudent is called with null', () {
      const student = Student(
        id: 1,
        studentId: 'STU001',
        firstName: 'John',
        lastName: 'Doe',
        email: 'john@example.com',
      );
      
      studentNotifier.selectStudent(student);
      studentNotifier.selectStudent(null);
      final studentState = container.read(studentProvider);
      expect(studentState.selectedStudent, isNull);
    });

    test('should clear error when clearError is called', () {
      final initialState = container.read(studentProvider);
      expect(initialState.error, isNull);
      
      studentNotifier.clearError();
      final updatedState = container.read(studentProvider);
      expect(updatedState.error, isNull);
    });
  });

  group('Student Model Tests', () {
    test('should create student with required fields', () {
      const student = Student(
        id: 1,
        studentId: 'STU001',
        firstName: 'John',
        lastName: 'Doe',
        email: 'john@example.com',
      );
      
      expect(student.id, equals(1));
      expect(student.studentId, equals('STU001'));
      expect(student.firstName, equals('John'));
      expect(student.lastName, equals('Doe'));
      expect(student.email, equals('john@example.com'));
    });

    test('should support equality', () {
      const student1 = Student(
        id: 1,
        studentId: 'STU001',
        firstName: 'John',
        lastName: 'Doe',
        email: 'john@example.com',
      );
      
      const student2 = Student(
        id: 1,
        studentId: 'STU001',
        firstName: 'John',
        lastName: 'Doe',
        email: 'john@example.com',
      );
      
      expect(student1, equals(student2));
    });

    test('should create copy with updated values', () {
      const student = Student(
        id: 1,
        studentId: 'STU001',
        firstName: 'John',
        lastName: 'Doe',
        email: 'john@example.com',
      );
      
      final updatedStudent = student.copyWith(firstName: 'Jane');
      
      expect(updatedStudent.id, equals(student.id));
      expect(updatedStudent.studentId, equals(student.studentId));
      expect(updatedStudent.firstName, equals('Jane'));
      expect(updatedStudent.lastName, equals(student.lastName));
    });

    test('should return correct full name', () {
      const student = Student(
        id: 1,
        studentId: 'STU001',
        firstName: 'John',
        lastName: 'Doe',
        email: 'john@example.com',
      );
      
      expect(student.fullName, equals('Doe, John'));
      expect(student.displayName, equals('John Doe'));
    });
  });
}