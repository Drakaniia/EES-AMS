import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import '../providers/auth_provider.dart';
import '../screens/auth/auth_screen.dart';
import '../screens/dashboard/dashboard_screen.dart';
import '../screens/attendance/attendance_screen.dart';
import '../screens/classes/classes_screen.dart';
import '../screens/students/students_screen.dart';
import '../screens/settings/settings_screen.dart';
import '../screens/profile/profile_settings_screen.dart';
import '../screens/splash/splash_screen.dart';

final appRouterProvider = Provider<GoRouter>((ref) {
  final authState = ref.watch(authStateProvider);

  return GoRouter(
    initialLocation: '/splash',
    redirect: (context, state) {
      final isAuthenticated = authState.isAuthenticated;

      // If going to auth route and already authenticated, redirect to dashboard
      if (isAuthenticated && state.uri.toString().startsWith('/auth')) {
        return '/dashboard';
      }

      // If going to protected route and not authenticated, redirect to auth
      if (!isAuthenticated &&
          !state.uri.toString().startsWith('/auth') &&
          !state.uri.toString().startsWith('/splash')) {
        return '/auth/login';
      }

      return null;
    },
    routes: [
      // Splash screen
      GoRoute(
        path: '/splash',
        name: 'splash',
        builder: (context, state) => const SplashScreen(),
      ),

      // Authentication routes
      GoRoute(
        path: '/auth',
        name: 'auth',
        builder: (context, state) => const AuthScreen(),
        routes: [
          GoRoute(
            path: '/login',
            name: 'login',
            builder: (context, state) => const AuthScreen(),
          ),
          GoRoute(
            path: '/register',
            name: 'register',
            builder: (context, state) => const AuthScreen(),
          ),
        ],
      ),

      // Main routes (protected)
      GoRoute(
        path: '/dashboard',
        name: 'dashboard',
        builder: (context, state) => const DashboardScreen(),
      ),

      GoRoute(
        path: '/attendance',
        name: 'attendance',
        builder: (context, state) => const AttendanceScreen(),
        routes: [
          GoRoute(
            path: '/class/:classId',
            name: 'attendance_by_class',
            builder: (context, state) {
              final classId = int.parse(state.pathParameters['classId']!);
              return AttendanceScreen(classId: classId);
            },
          ),
          GoRoute(
            path: '/date/:date',
            name: 'attendance_by_date',
            builder: (context, state) {
              final date = DateTime.parse(state.pathParameters['date']!);
              return AttendanceScreen(date: date);
            },
          ),
        ],
      ),

      GoRoute(
        path: '/classes',
        name: 'classes',
        builder: (context, state) => const ClassesScreen(),
        routes: [
          GoRoute(
            path: '/:id',
            name: 'class_detail',
            builder: (context, state) {
              final classId = int.parse(state.pathParameters['id']!);
              return ClassesScreen(selectedClassId: classId);
            },
          ),
          GoRoute(
            path: '/new',
            name: 'new_class',
            builder: (context, state) => const ClassesScreen(isAddingNew: true),
          ),
        ],
      ),

      GoRoute(
        path: '/students',
        name: 'students',
        builder: (context, state) => const StudentsScreen(),
        routes: [
          GoRoute(
            path: '/class/:classId',
            name: 'students_by_class',
            builder: (context, state) {
              final classId = int.parse(state.pathParameters['classId']!);
              return StudentsScreen(filterClassId: classId);
            },
          ),
          GoRoute(
            path: '/:id',
            name: 'student_detail',
            builder: (context, state) {
              final studentId = int.parse(state.pathParameters['id']!);
              return StudentsScreen(selectedStudentId: studentId);
            },
          ),
          GoRoute(
            path: '/new',
            name: 'new_student',
            builder: (context, state) =>
                const StudentsScreen(isAddingNew: true),
          ),
        ],
      ),

      GoRoute(
        path: '/settings',
        name: 'settings',
        builder: (context, state) => const SettingsScreen(),
        routes: [
          GoRoute(
            path: '/profile',
            name: 'profile_settings',
            builder: (context, state) => const ProfileSettingsScreen(),
          ),
          GoRoute(
            path: '/sync',
            name: 'sync_settings',
            builder: (context, state) => const SettingsScreen(initialTab: 1),
          ),
          GoRoute(
            path: '/updates',
            name: 'update_settings',
            builder: (context, state) => const SettingsScreen(initialTab: 2),
          ),
        ],
      ),
    ],

    // Error handling
    errorBuilder: (context, state) => Scaffold(
      body: Center(
        child: Column(
          mainAxisAlignment: MainAxisAlignment.center,
          children: [
            Text('Error: ${state.error}'),
            const SizedBox(height: 16),
            ElevatedButton(
              onPressed: () => context.go('/dashboard'),
              child: const Text('Go to Dashboard'),
            ),
          ],
        ),
      ),
    ),

    // Debug logging
    debugLogDiagnostics: true,
  );
});
