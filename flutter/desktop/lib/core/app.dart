import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import '../providers/auth_provider.dart';
import '../routes/app_router.dart';
import '../widgets/layout/app_layout.dart';
import '../services/api_client.dart';

class App extends ConsumerWidget {
  const App({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final authState = ref.watch(authStateProvider);

    // Set auth token for API client
    final apiClient = ref.read(apiClientProvider);
    if (authState.token != null) {
      apiClient.setAuthToken(authState.token!);
    }

    return AppLayout(
      child: MaterialApp.router(
        routerConfig: ref.watch(appRouterProvider),
        title: 'AttendEase - AMS',
        debugShowCheckedModeBanner: false,
      ),
    );
  }
}
