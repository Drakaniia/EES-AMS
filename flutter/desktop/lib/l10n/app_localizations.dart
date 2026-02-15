/// Helper class to provide app localizations
class AppLocalizations {
  const AppLocalizations();

  static const AppLocalizations instance = AppLocalizations();

  String get appName => 'AttendEase';
  String get attendanceSystem => 'Attendance Management System';

  // Common
  String get loading => 'Loading...';
  String get error => 'Error';
  String get success => 'Success';
  String get cancel => 'Cancel';
  String get confirm => 'Confirm';
  String get save => 'Save';
  String get delete => 'Delete';
  String get edit => 'Edit';
  String get add => 'Add';
  String get search => 'Search';
  String get filter => 'Filter';

  // Auth
  String get signIn => 'Sign In';
  String get signUp => 'Sign Up';
  String get signOut => 'Sign Out';
  String get email => 'Email';
  String get password => 'Password';
  String get confirmPassword => 'Confirm Password';
  String get fullName => 'Full Name';
  String get schoolName => 'School Name';
  String get forgotPassword => 'Forgot Password?';
  String get createAccount => 'Create Account';
  String get alreadyHaveAccount => 'Already have an account?';
  String get dontHaveAccount => "Don't have an account?";

  // Navigation
  String get dashboard => 'Dashboard';
  String get attendance => 'Attendance';
  String get classes => 'Classes';
  String get students => 'Students';
  String get settings => 'Settings';
  String get profile => 'Profile';

  // Dashboard
  String get welcome => 'Welcome';
  String get goodMorning => 'Good morning';
  String get goodAfternoon => 'Good afternoon';
  String get goodEvening => 'Good evening';
  String get totalClasses => 'Total Classes';
  String get totalStudents => 'Total Students';
  String get presentToday => 'Present Today';
  String get absentToday => 'Absent Today';
  String get overview => 'Overview';
  String get quickActions => 'Quick Actions';
  String get recentActivity => 'Recent Activity';
  String get takeAttendance => 'Take Attendance';
  String get addStudent => 'Add Student';
  String get createClass => 'Create Class';

  // Classes
  String get className => 'Class Name';
  String get section => 'Section';
  String get schoolYear => 'School Year';
  String get newClass => 'New Class';
  String get editClass => 'Edit Class';
  String get deleteClass => 'Delete Class';

  // Students
  String get studentId => 'Student ID';
  String get firstName => 'First Name';
  String get lastName => 'Last Name';
  String get newStudent => 'New Student';
  String get editStudent => 'Edit Student';
  String get deleteStudent => 'Delete Student';
  String get importFromExcel => 'Import from Excel';

  // Attendance
  String get date => 'Date';
  String get present => 'Present';
  String get absent => 'Absent';
  String get late => 'Late';
  String get excused => 'Excused';
  String get allPresent => 'All Present';
  String get allAbsent => 'All Absent';
  String get markAttendance => 'Mark Attendance';
  String get attendanceRate => 'Attendance Rate';

  // Settings
  String get generalSettings => 'General Settings';
  String get syncSettings => 'Sync Settings';
  String get updateSettings => 'Update Settings';
  String get darkMode => 'Dark Mode';
  String get language => 'Language';
  String get connected => 'Connected';
  String get version => 'Version';

  // Messages
  String get loginSuccessful => 'Login successful';
  String get loginFailed => 'Login failed';
  String get registrationSuccessful => 'Registration successful';
  String get registrationFailed => 'Registration failed';
  String get somethingWentWrong => 'Something went wrong';
  String get noDataAvailable => 'No data available';
  String get confirmDelete => 'Are you sure you want to delete this item?';
  String get confirmLogout => 'Are you sure you want to logout?';
}