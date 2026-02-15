import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import '../../widgets/layout/app_layout.dart';
import '../../themes/app_theme.dart';
import '../../widgets/common/loading_widget.dart';
import '../../widgets/cards/stats_card.dart';
import '../../widgets/cards/quick_action_card.dart';
import '../../providers/class_provider.dart';
import '../../providers/student_provider.dart';
import '../../providers/attendance_provider.dart';
import '../../core/providers/auth_providers.dart';

class DashboardScreen extends ConsumerStatefulWidget {
  const DashboardScreen({super.key});

  @override
  ConsumerState<DashboardScreen> createState() => _DashboardScreenState();
}

class _DashboardScreenState extends ConsumerState<DashboardScreen>
    with AutomaticKeepAliveClientMixin {
  @override
  bool get wantKeepAlive => true;

  @override
  Widget build(BuildContext context) {
    super.build(context);

    final classState = ref.watch(classProvider);
    final studentState = ref.watch(studentProvider);
    final attendanceState = ref.watch(attendanceProvider);

    return AppLayout(
      title: 'Dashboard',
      showSidebar: true,
      child: RefreshIndicator(
        onRefresh: _refreshAllData,
        child: SingleChildScrollView(
          padding: const EdgeInsets.all(AppSpaces.lg),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              // Welcome section
              _buildWelcomeSection(),
              const SizedBox(height: AppSpaces.xl),

              // Stats cards
              _buildStatsSection(classState, studentState, attendanceState),
              const SizedBox(height: AppSpaces.xl),

              // Quick actions
              _buildQuickActionsSection(),
              const SizedBox(height: AppSpaces.xl),

              // Recent activity
              _buildRecentActivitySection(),
            ],
          ),
        ),
      ),
    );
  }

  Widget _buildWelcomeSection() {
    final authState = ref.watch(authStateProvider);
    final hour = DateTime.now().hour;
    String greeting = 'Good morning';
    if (hour >= 12 && hour < 17) {
      greeting = 'Good afternoon';
    } else if (hour >= 17) {
      greeting = 'Good evening';
    }

    return Container(
      padding: const EdgeInsets.all(AppSpaces.lg),
      decoration: BoxDecoration(
        gradient: LinearGradient(
          begin: Alignment.topLeft,
          end: Alignment.bottomRight,
          colors: [
            AppColors.primary,
            AppColors.primaryVariant,
          ],
        ),
        borderRadius: BorderRadius.circular(AppRadius.lg),
        boxShadow: [
          BoxShadow(
            color: AppColors.primary.withValues(alpha: 0.3),
            blurRadius: 20,
            offset: const Offset(0, 8),
          ),
        ],
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(
            '$greeting, ${authState.user?.displayName ?? 'User'}!',
            style: Theme.of(context).textTheme.displaySmall?.copyWith(
                  color: Colors.white,
                  fontWeight: FontWeight.bold,
                ),
          ),
          const SizedBox(height: AppSpaces.sm),
          Text(
            'Here\'s what\'s happening with your attendance today.',
            style: Theme.of(context).textTheme.bodyLarge?.copyWith(
                  color: Colors.white.withValues(alpha: 0.9),
                ),
          ),
          const SizedBox(height: AppSpaces.md),
          Text(
            DateTime.now().toString().substring(0, 10),
            style: Theme.of(context).textTheme.bodySmall?.copyWith(
                  color: Colors.white.withValues(alpha: 0.8),
                ),
          ),
        ],
      ),
    );
  }

  Widget _buildStatsSection(
    ClassState classState,
    StudentState studentState,
    AttendanceState attendanceState,
  ) {
    if (classState.isLoading ||
        studentState.isLoading ||
        attendanceState.isLoading) {
      return const SizedBox(
        height: 150,
        child: LoadingWidget(message: 'Loading statistics...'),
      );
    }

    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text(
          'Overview',
          style: Theme.of(context).textTheme.headlineMedium?.copyWith(
                fontWeight: FontWeight.bold,
              ),
        ),
        const SizedBox(height: AppSpaces.md),
        GridView.count(
          shrinkWrap: true,
          physics: const NeverScrollableScrollPhysics(),
          crossAxisCount: 4,
          crossAxisSpacing: AppSpaces.md,
          mainAxisSpacing: AppSpaces.md,
          childAspectRatio: 1.5,
          children: [
            StatsCard(
              title: 'Total Classes',
              value: classState.classes.length.toString(),
              icon: Icons.class_outlined,
              color: AppColors.primary,
              trend: '+2 from last month',
            ),
            StatsCard(
              title: 'Total Students',
              value: studentState.students.length.toString(),
              icon: Icons.people_outline,
              color: AppColors.secondary,
              trend: '+5 from last month',
            ),
            StatsCard(
              title: 'Present Today',
              value: attendanceState.stats?.presentToday.toString() ?? '0',
              icon: Icons.check_circle_outline,
              color: AppColors.success,
              trend:
                  '${attendanceState.stats?.attendanceRate.toStringAsFixed(1) ?? '0'}% rate',
            ),
            StatsCard(
              title: 'Absent Today',
              value: attendanceState.stats?.absentToday.toString() ?? '0',
              icon: Icons.cancel_outlined,
              color: AppColors.error,
              trend: 'Need attention',
            ),
          ],
        ),
      ],
    );
  }

  Widget _buildQuickActionsSection() {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text(
          'Quick Actions',
          style: Theme.of(context).textTheme.headlineMedium?.copyWith(
                fontWeight: FontWeight.bold,
              ),
        ),
        const SizedBox(height: AppSpaces.md),
        GridView.count(
          shrinkWrap: true,
          physics: const NeverScrollableScrollPhysics(),
          crossAxisCount: 3,
          crossAxisSpacing: AppSpaces.md,
          mainAxisSpacing: AppSpaces.md,
          childAspectRatio: 1.2,
          children: [
            QuickActionCard(
              title: 'Take Attendance',
              subtitle: 'Mark student attendance',
              icon: Icons.how_to_reg,
              color: AppColors.primary,
              onTap: () => GoRouter.of(context).go('/attendance'),
            ),
            QuickActionCard(
              title: 'Add Student',
              subtitle: 'Register new student',
              icon: Icons.person_add,
              color: AppColors.secondary,
              onTap: () => GoRouter.of(context).go('/students/new'),
            ),
            QuickActionCard(
              title: 'Create Class',
              subtitle: 'Setup new class',
              icon: Icons.class_outlined,
              color: AppColors.warning,
              onTap: () => GoRouter.of(context).go('/classes/new'),
            ),
          ],
        ),
      ],
    );
  }

  Widget _buildRecentActivitySection() {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text(
          'Recent Activity',
          style: Theme.of(context).textTheme.headlineMedium?.copyWith(
                fontWeight: FontWeight.bold,
              ),
        ),
        const SizedBox(height: AppSpaces.md),
        Container(
          padding: const EdgeInsets.all(AppSpaces.lg),
          decoration: BoxDecoration(
            color: Theme.of(context).colorScheme.surface,
            borderRadius: BorderRadius.circular(AppRadius.lg),
            border: Border.all(color: AppColors.border),
          ),
          child: Column(
            children: [
              _buildActivityItem(
                icon: Icons.how_to_reg,
                title: 'Attendance Completed',
                subtitle: 'Class 10A - 25 students marked present',
                time: '2 hours ago',
                color: AppColors.success,
              ),
              const Divider(),
              _buildActivityItem(
                icon: Icons.person_add,
                title: 'New Student Added',
                subtitle: 'John Doe joined Class 9B',
                time: '5 hours ago',
                color: AppColors.primary,
              ),
              const Divider(),
              _buildActivityItem(
                icon: Icons.class_outlined,
                title: 'Class Created',
                subtitle: 'New class "Science 11A" created',
                time: '1 day ago',
                color: AppColors.secondary,
              ),
            ],
          ),
        ),
      ],
    );
  }

  Widget _buildActivityItem({
    required IconData icon,
    required String title,
    required String subtitle,
    required String time,
    required Color color,
  }) {
    return ListTile(
      leading: Container(
        padding: const EdgeInsets.all(8),
        decoration: BoxDecoration(
          color: color.withValues(alpha: 0.1),
          borderRadius: BorderRadius.circular(AppRadius.md),
        ),
        child: Icon(icon, color: color, size: 20),
      ),
      title: Text(
        title,
        style: Theme.of(context).textTheme.labelMedium?.copyWith(
              fontWeight: FontWeight.w600,
            ),
      ),
      subtitle: Text(subtitle),
      trailing: Text(
        time,
        style: Theme.of(context).textTheme.bodySmall?.copyWith(
              color: AppColors.textSecondary,
            ),
      ),
    );
  }

  Future<void> _refreshAllData() async {
    await Future.wait([
      ref.read(classProvider.notifier).loadClasses(),
      ref.read(studentProvider.notifier).loadStudents(),
      ref.read(attendanceProvider.notifier).loadStats(),
    ]);
  }
}
