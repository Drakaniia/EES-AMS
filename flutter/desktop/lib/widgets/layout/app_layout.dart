import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import '../../providers/auth_provider.dart';
import '../../themes/app_theme.dart';
import 'sidebar_widget.dart';

class AppLayout extends ConsumerWidget {
  final Widget child;
  final bool showSidebar;
  final String? title;
  final List<Widget>? actions;

  const AppLayout({
    super.key,
    required this.child,
    this.showSidebar = true,
    this.title,
    this.actions,
  });

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final authState = ref.watch(authStateProvider);
    final currentIndex = _getCurrentIndex(context);

    // Don't show sidebar for auth and splash screens
    final shouldShowSidebar = showSidebar && authState.isAuthenticated;

    if (!shouldShowSidebar) {
      return Scaffold(
        body: child,
      );
    }

    return Scaffold(
      body: Row(
        children: [
          // Sidebar
          SidebarWidget(
            currentIndex: currentIndex,
            onItemSelected: (index) {
              _navigateToPage(context, index);
            },
          ),

          // Main content
          Expanded(
            child: Column(
              children: [
                // App bar
                if (title != null)
                  Container(
                    padding: const EdgeInsets.all(AppSpaces.md),
                    decoration: BoxDecoration(
                      color: Theme.of(context).colorScheme.surface,
                      border: Border(
                        bottom: BorderSide(
                          color: Theme.of(context).dividerColor,
                          width: 1,
                        ),
                      ),
                    ),
                    child: Row(
                      children: [
                        if (title != null) ...[
                          Expanded(
                            child: Text(
                              title!,
                              style: Theme.of(context).textTheme.headlineMedium,
                            ),
                          ),
                        ],
                        if (actions != null) ...actions!,
                        PopupMenuButton<String>(
                          icon: const Icon(Icons.more_vert),
                          onSelected: (value) {
                            _handleMenuAction(context, value);
                          },
                          itemBuilder: (context) => [
                            const PopupMenuItem(
                              value: 'profile',
                              child: Row(
                                children: [
                                  Icon(Icons.person_outline),
                                  SizedBox(width: AppSpaces.sm),
                                  Text('Profile'),
                                ],
                              ),
                            ),
                            const PopupMenuItem(
                              value: 'settings',
                              child: Row(
                                children: [
                                  Icon(Icons.settings_outlined),
                                  SizedBox(width: AppSpaces.sm),
                                  Text('Settings'),
                                ],
                              ),
                            ),
                            const PopupMenuItem(
                              value: 'about',
                              child: Row(
                                children: [
                                  Icon(Icons.info_outline),
                                  SizedBox(width: AppSpaces.sm),
                                  Text('About'),
                                ],
                              ),
                            ),
                            const PopupMenuItem(
                              value: 'logout',
                              child: Row(
                                children: [
                                  Icon(Icons.logout, color: AppColors.error),
                                  SizedBox(width: AppSpaces.sm),
                                  Text('Logout',
                                      style: TextStyle(color: AppColors.error)),
                                ],
                              ),
                            ),
                          ],
                        ),
                      ],
                    ),
                  ),

                // Page content
                Expanded(
                  child: child,
                ),
              ],
            ),
          ),
        ],
      ),
    );
  }

  int _getCurrentIndex(BuildContext context) {
    final location = GoRouterState.of(context).uri.toString();
    if (location.startsWith('/dashboard')) return 0;
    if (location.startsWith('/attendance')) return 1;
    if (location.startsWith('/classes')) return 2;
    if (location.startsWith('/students')) return 3;
    if (location.startsWith('/settings')) return 4;
    return 0;
  }

  void _navigateToPage(BuildContext context, int index) {
    switch (index) {
      case 0:
        context.go('/dashboard');
        break;
      case 1:
        context.go('/attendance');
        break;
      case 2:
        context.go('/classes');
        break;
      case 3:
        context.go('/students');
        break;
      case 4:
        context.go('/settings');
        break;
    }
  }

  void _handleMenuAction(BuildContext context, String action) {
    switch (action) {
      case 'profile':
        context.go('/settings/profile');
        break;
      case 'settings':
        context.go('/settings');
        break;
      case 'about':
        _showAboutDialog(context);
        break;
      case 'logout':
        _showLogoutDialog(context);
        break;
    }
  }

  void _showAboutDialog(BuildContext context) {
    showDialog(
      context: context,
      builder: (context) => AlertDialog(
        title: const Text('About AttendEase'),
        content: const Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text('AttendEase Attendance Management System'),
            SizedBox(height: AppSpaces.sm),
            Text('Version: 1.0.0'),
            SizedBox(height: AppSpaces.sm),
            Text('Built with Flutter and ❤️'),
          ],
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.of(context).pop(),
            child: const Text('Close'),
          ),
        ],
      ),
    );
  }

  void _showLogoutDialog(BuildContext context) {
    showDialog(
      context: context,
      builder: (context) => AlertDialog(
        title: const Text('Logout'),
        content: const Text('Are you sure you want to logout?'),
        actions: [
          TextButton(
            onPressed: () => Navigator.of(context).pop(),
            child: const Text('Cancel'),
          ),
          ElevatedButton(
            onPressed: () {
              Navigator.of(context).pop();
              // Handle logout
              ProviderScope.containerOf(context)
                  .read(authStateProvider.notifier)
                  .logout();
            },
            style: ElevatedButton.styleFrom(
              backgroundColor: AppColors.error,
              foregroundColor: Colors.white,
            ),
            child: const Text('Logout'),
          ),
        ],
      ),
    );
  }
}
