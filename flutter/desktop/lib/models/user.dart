import 'package:equatable/equatable.dart';
import 'package:json_annotation/json_annotation.dart';

part 'user.g.dart';

@JsonSerializable()
class User extends Equatable {
  final int? id;
  @JsonKey(name: 'email')
  final String email;
  @JsonKey(name: 'display_name')
  final String displayName;
  @JsonKey(name: 'school_name')
  final String? schoolName;
  @JsonKey(name: 'role')
  final String role;
  @JsonKey(name: 'is_active')
  final bool isActive;
  @JsonKey(name: 'created_at')
  final DateTime? createdAt;
  @JsonKey(name: 'updated_at')
  final DateTime? updatedAt;
  @JsonKey(name: 'last_login')
  final DateTime? lastLogin;

  const User({
    this.id,
    required this.email,
    required this.displayName,
    this.schoolName,
    required this.role,
    this.isActive = true,
    this.createdAt,
    this.updatedAt,
    this.lastLogin,
  });

  factory User.fromJson(Map<String, dynamic> json) => _$UserFromJson(json);
  Map<String, dynamic> toJson() => _$UserToJson(this);

  User copyWith({
    int? id,
    String? email,
    String? displayName,
    String? schoolName,
    String? role,
    bool? isActive,
    DateTime? createdAt,
    DateTime? updatedAt,
    DateTime? lastLogin,
  }) {
    return User(
      id: id ?? this.id,
      email: email ?? this.email,
      displayName: displayName ?? this.displayName,
      schoolName: schoolName ?? this.schoolName,
      role: role ?? this.role,
      isActive: isActive ?? this.isActive,
      createdAt: createdAt ?? this.createdAt,
      updatedAt: updatedAt ?? this.updatedAt,
      lastLogin: lastLogin ?? this.lastLogin,
    );
  }

  @override
  List<Object?> get props => [
        id,
        email,
        displayName,
        schoolName,
        role,
        isActive,
        createdAt,
        updatedAt,
        lastLogin,
      ];
}

@JsonSerializable()
class AuthCredentials extends Equatable {
  @JsonKey(name: 'email')
  final String email;
  @JsonKey(name: 'password')
  final String password;

  const AuthCredentials({
    required this.email,
    required this.password,
  });

  factory AuthCredentials.fromJson(Map<String, dynamic> json) =>
      _$AuthCredentialsFromJson(json);
  Map<String, dynamic> toJson() => _$AuthCredentialsToJson(this);

  @override
  List<Object?> get props => [email, password];
}

@JsonSerializable()
class AuthResponse extends Equatable {
  @JsonKey(name: 'user')
  final User user;
  @JsonKey(name: 'token')
  final String token;
  @JsonKey(name: 'refresh_token')
  final String refreshToken;
  @JsonKey(name: 'expires_in')
  final int expiresIn;

  const AuthResponse({
    required this.user,
    required this.token,
    required this.refreshToken,
    required this.expiresIn,
  });

  factory AuthResponse.fromJson(Map<String, dynamic> json) =>
      _$AuthResponseFromJson(json);
  Map<String, dynamic> toJson() => _$AuthResponseToJson(this);

  @override
  List<Object?> get props => [user, token, refreshToken, expiresIn];
}