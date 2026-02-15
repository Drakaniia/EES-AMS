import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:google_sign_in/google_sign_in.dart';

import 'package:flutter_secure_storage/flutter_secure_storage.dart';

import '../../providers/auth_provider.dart';
import '../../themes/app_theme.dart';
import '../../widgets/common/logo_widget.dart';
import '../../widgets/common/custom_text_field.dart';
import '../../widgets/common/custom_button.dart';

class AuthScreen extends ConsumerStatefulWidget {
  const AuthScreen({super.key});

  @override
  ConsumerState<AuthScreen> createState() => _AuthScreenState();
}

class _AuthScreenState extends ConsumerState<AuthScreen>
    with TickerProviderStateMixin {
  late TabController _tabController;
  final GoogleSignIn _googleSignIn = GoogleSignIn(
    scopes: [
      'email',
      'https://www.googleapis.com/auth/contacts.readonly',
    ],
  );

  final _secureStorage = const FlutterSecureStorage();

  // Login form controllers
  final _loginEmailController = TextEditingController();
  final _loginPasswordController = TextEditingController();
  final _loginFormKey = GlobalKey<FormState>();

  // Register form controllers
  final _registerEmailController = TextEditingController();
  final _registerPasswordController = TextEditingController();
  final _registerConfirmPasswordController = TextEditingController();
  final _registerNameController = TextEditingController();
  final _registerSchoolController = TextEditingController();
  final _registerFormKey = GlobalKey<FormState>();

  bool _obscurePassword = true;
  bool _obscureConfirmPassword = true;

  @override
  void initState() {
    super.initState();
    _tabController = TabController(length: 2, vsync: this);
    _tabController.addListener(() {
      setState(() {});
    });
  }

  @override
  void dispose() {
    _tabController.dispose();
    _loginEmailController.dispose();
    _loginPasswordController.dispose();
    _registerEmailController.dispose();
    _registerPasswordController.dispose();
    _registerConfirmPasswordController.dispose();
    _registerNameController.dispose();
    _registerSchoolController.dispose();
    super.dispose();
  }

  Future<void> _handleLogin() async {
    if (!_loginFormKey.currentState!.validate()) return;

    final authNotifier = ref.read(authStateProvider.notifier);
    await authNotifier.login(
      _loginEmailController.text.trim(),
      _loginPasswordController.text,
    );
  }

  Future<void> _handleRegister() async {
    if (!_registerFormKey.currentState!.validate()) return;

    final authNotifier = ref.read(authStateProvider.notifier);
    await authNotifier.register({
      'email': _registerEmailController.text.trim(),
      'password': _registerPasswordController.text,
      'display_name': _registerNameController.text.trim(),
      'school_name': _registerSchoolController.text.trim(),
    });
  }

  Future<void> _handleGoogleSignIn() async {
    try {
      // Trigger the authentication flow
      final GoogleSignInAccount? googleUser = await _googleSignIn.signIn();

      if (googleUser == null) {
        // User cancelled the sign-in
        return;
      }

      // Obtain the auth details from the request
      final GoogleSignInAuthentication googleAuth =
          await googleUser.authentication;

      // Store authentication tokens for future use
      await _secureStorage.write(
          key: 'google_access_token', value: googleAuth.accessToken);
      await _secureStorage.write(
          key: 'google_id_token', value: googleAuth.idToken);

      // Store user information
      await _secureStorage.write(key: 'user_email', value: googleUser.email);
      await _secureStorage.write(
          key: 'user_name', value: googleUser.displayName ?? '');
      await _secureStorage.write(key: 'user_id', value: googleUser.id);

      // Use idToken as auth token for the application
      if (googleAuth.idToken != null) {
        await _secureStorage.write(
            key: 'auth_token', value: googleAuth.idToken!);

        // Update the app's authentication state
        final authNotifier = ref.read(authStateProvider.notifier);
        await authNotifier.signInWithGoogle({
          'user': {
            'email': googleUser.email,
            'name': googleUser.displayName,
            'id': googleUser.id,
          },
          'tokens': {
            'accessToken': googleAuth.accessToken,
            'idToken': googleAuth.idToken,
          },
        });
      }
    } catch (error) {
      // Handle authentication errors
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(
            content: Text('Google Sign-In failed: ${error.toString()}'),
            backgroundColor: Colors.red,
          ),
        );
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    final authState = ref.watch(authStateProvider);

    return Scaffold(
      body: SafeArea(
        child: Center(
          child: SingleChildScrollView(
            padding: const EdgeInsets.all(AppSpaces.lg),
            child: ConstrainedBox(
              constraints: const BoxConstraints(maxWidth: 400),
              child: Column(
                mainAxisAlignment: MainAxisAlignment.center,
                children: [
                  const LogoWidget(size: 80),
                  const SizedBox(height: AppSpaces.lg),
                  Text(
                    'AttendEase',
                    style: Theme.of(context).textTheme.displaySmall?.copyWith(
                          color: AppColors.primary,
                          fontWeight: FontWeight.bold,
                        ),
                  ),
                  const SizedBox(height: AppSpaces.sm),
                  Text(
                    'Attendance Management System',
                    style: Theme.of(context).textTheme.bodyMedium?.copyWith(
                          color: AppColors.textSecondary,
                        ),
                  ),
                  const SizedBox(height: AppSpaces.xl),

                  // Tab bar for Login/Register
                  Container(
                    decoration: BoxDecoration(
                      color: Theme.of(context).colorScheme.surface,
                      borderRadius: BorderRadius.circular(AppRadius.lg),
                      border: Border.all(color: AppColors.border),
                    ),
                    child: TabBar(
                      controller: _tabController,
                      indicator: BoxDecoration(
                        color: AppColors.primary,
                        borderRadius: BorderRadius.circular(AppRadius.md),
                      ),
                      labelColor: Colors.white,
                      unselectedLabelColor: AppColors.textSecondary,
                      tabs: const [
                        Tab(text: 'Login'),
                        Tab(text: 'Register'),
                      ],
                    ),
                  ),
                  const SizedBox(height: AppSpaces.lg),

                  // Tab views
                  SizedBox(
                    height: 400,
                    child: TabBarView(
                      controller: _tabController,
                      children: [
                        _buildLoginTab(authState),
                        _buildRegisterTab(authState),
                      ],
                    ),
                  ),

                  // Error message
                  if (authState.error != null) ...[
                    const SizedBox(height: AppSpaces.md),
                    Container(
                      padding: const EdgeInsets.all(AppSpaces.md),
                      decoration: BoxDecoration(
                        color: AppColors.error.withValues(alpha: 0.1),
                        border: Border.all(color: AppColors.error),
                        borderRadius: BorderRadius.circular(AppRadius.md),
                      ),
                      child: Row(
                        children: [
                          Icon(Icons.error_outline, color: AppColors.error),
                          const SizedBox(width: AppSpaces.sm),
                          Expanded(
                            child: Text(
                              authState.error!,
                              style: TextStyle(color: AppColors.error),
                            ),
                          ),
                        ],
                      ),
                    ),
                  ],
                ],
              ),
            ),
          ),
        ),
      ),
    );
  }

  Widget _buildLoginTab(AuthState authState) {
    return Form(
      key: _loginFormKey,
      child: Column(
        children: [
          CustomTextField(
            controller: _loginEmailController,
            label: 'Email',
            keyboardType: TextInputType.emailAddress,
            prefixIcon: Icons.email_outlined,
            validator: (value) {
              if (value == null || value.isEmpty) {
                return 'Please enter your email';
              }
              if (!RegExp(r'^[\w-\.]+@([\w-]+\.)+[\w-]{2,4}$')
                  .hasMatch(value)) {
                return 'Please enter a valid email';
              }
              return null;
            },
          ),
          const SizedBox(height: AppSpaces.md),
          CustomTextField(
            controller: _loginPasswordController,
            label: 'Password',
            obscureText: _obscurePassword,
            prefixIcon: Icons.lock_outline,
            suffixIcon: IconButton(
              icon: Icon(
                _obscurePassword ? Icons.visibility_off : Icons.visibility,
              ),
              onPressed: () {
                setState(() {
                  _obscurePassword = !_obscurePassword;
                });
              },
            ),
            validator: (value) {
              if (value == null || value.isEmpty) {
                return 'Please enter your password';
              }
              if (value.length < 6) {
                return 'Password must be at least 6 characters';
              }
              return null;
            },
          ),
          const SizedBox(height: AppSpaces.lg),
          CustomButton(
            text: 'Sign In',
            onPressed: authState.isLoading ? null : _handleLogin,
            isLoading: authState.isLoading,
          ),
          const SizedBox(height: AppSpaces.md),
          CustomButton(
            text: 'Sign in with Google',
            onPressed: authState.isLoading ? null : _handleGoogleSignIn,
            variant: ButtonVariant.outlined,
            prefixIcon: Icons.g_mobiledata,
          ),
        ],
      ),
    );
  }

  Widget _buildRegisterTab(AuthState authState) {
    return Form(
      key: _registerFormKey,
      child: SingleChildScrollView(
        child: Column(
          children: [
            CustomTextField(
              controller: _registerNameController,
              label: 'Full Name',
              prefixIcon: Icons.person_outline,
              validator: (value) {
                if (value == null || value.isEmpty) {
                  return 'Please enter your full name';
                }
                return null;
              },
            ),
            const SizedBox(height: AppSpaces.md),
            CustomTextField(
              controller: _registerEmailController,
              label: 'Email',
              keyboardType: TextInputType.emailAddress,
              prefixIcon: Icons.email_outlined,
              validator: (value) {
                if (value == null || value.isEmpty) {
                  return 'Please enter your email';
                }
                if (!RegExp(r'^[\w-\.]+@([\w-]+\.)+[\w-]{2,4}$')
                    .hasMatch(value)) {
                  return 'Please enter a valid email';
                }
                return null;
              },
            ),
            const SizedBox(height: AppSpaces.md),
            CustomTextField(
              controller: _registerSchoolController,
              label: 'School Name',
              prefixIcon: Icons.school_outlined,
              validator: (value) {
                if (value == null || value.isEmpty) {
                  return 'Please enter your school name';
                }
                return null;
              },
            ),
            const SizedBox(height: AppSpaces.md),
            CustomTextField(
              controller: _registerPasswordController,
              label: 'Password',
              obscureText: _obscurePassword,
              prefixIcon: Icons.lock_outline,
              suffixIcon: IconButton(
                icon: Icon(
                  _obscurePassword ? Icons.visibility_off : Icons.visibility,
                ),
                onPressed: () {
                  setState(() {
                    _obscurePassword = !_obscurePassword;
                  });
                },
              ),
              validator: (value) {
                if (value == null || value.isEmpty) {
                  return 'Please enter a password';
                }
                if (value.length < 6) {
                  return 'Password must be at least 6 characters';
                }
                return null;
              },
            ),
            const SizedBox(height: AppSpaces.md),
            CustomTextField(
              controller: _registerConfirmPasswordController,
              label: 'Confirm Password',
              obscureText: _obscureConfirmPassword,
              prefixIcon: Icons.lock_outline,
              suffixIcon: IconButton(
                icon: Icon(
                  _obscureConfirmPassword
                      ? Icons.visibility_off
                      : Icons.visibility,
                ),
                onPressed: () {
                  setState(() {
                    _obscureConfirmPassword = !_obscureConfirmPassword;
                  });
                },
              ),
              validator: (value) {
                if (value == null || value.isEmpty) {
                  return 'Please confirm your password';
                }
                if (value != _registerPasswordController.text) {
                  return 'Passwords do not match';
                }
                return null;
              },
            ),
            const SizedBox(height: AppSpaces.lg),
            CustomButton(
              text: 'Create Account',
              onPressed: authState.isLoading ? null : _handleRegister,
              isLoading: authState.isLoading,
            ),
          ],
        ),
      ),
    );
  }
}
