import 'package:flutter/material.dart';
import '../../themes/app_theme.dart';

enum ButtonVariant { filled, outlined, text }

class CustomButton extends StatelessWidget {
  final String text;
  final VoidCallback? onPressed;
  final bool isLoading;
  final ButtonVariant variant;
  final IconData? prefixIcon;
  final IconData? suffixIcon;
  final Color? color;
  final Color? textColor;
  final double? width;
  final double? height;

  const CustomButton({
    super.key,
    required this.text,
    this.onPressed,
    this.isLoading = false,
    this.variant = ButtonVariant.filled,
    this.prefixIcon,
    this.suffixIcon,
    this.color,
    this.textColor,
    this.width,
    this.height,
  });

  @override
  Widget build(BuildContext context) {
    Widget child;
    
    switch (variant) {
      case ButtonVariant.filled:
        child = ElevatedButton(
          onPressed: isLoading ? null : onPressed,
          style: ElevatedButton.styleFrom(
            backgroundColor: color,
            foregroundColor: textColor,
            minimumSize: Size(width ?? double.infinity, height ?? 48),
          ),
          child: _buildButtonContent(),
        );
        break;
      case ButtonVariant.outlined:
        child = OutlinedButton(
          onPressed: isLoading ? null : onPressed,
          style: OutlinedButton.styleFrom(
            side: BorderSide(color: color ?? Theme.of(context).colorScheme.primary),
            minimumSize: Size(width ?? double.infinity, height ?? 48),
          ),
          child: _buildButtonContent(),
        );
        break;
      case ButtonVariant.text:
        child = TextButton(
          onPressed: isLoading ? null : onPressed,
          style: TextButton.styleFrom(
            minimumSize: Size(width ?? double.infinity, height ?? 48),
          ),
          child: _buildButtonContent(),
        );
        break;
    }

    return child;
  }

  Widget _buildButtonContent() {
    if (isLoading) {
      return const SizedBox(
        height: 20,
        width: 20,
        child: CircularProgressIndicator(
          strokeWidth: 2,
          valueColor: AlwaysStoppedAnimation<Color>(Colors.white),
        ),
      );
    }

    return Row(
      mainAxisSize: MainAxisSize.min,
      mainAxisAlignment: MainAxisAlignment.center,
      children: [
        if (prefixIcon != null) ...[
          Icon(prefixIcon, size: 20),
          const SizedBox(width: AppSpaces.sm),
        ],
        Text(text),
        if (suffixIcon != null) ...[
          const SizedBox(width: AppSpaces.sm),
          Icon(suffixIcon, size: 20),
        ],
      ],
    );
  }
}