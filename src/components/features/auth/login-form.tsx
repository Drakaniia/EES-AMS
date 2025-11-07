"use client";

import { useState } from "react";
import { useForm, Controller } from "react-hook-form";
import { zodResolver } from "@hookform/resolvers/zod";
import { motion } from "framer-motion";
import { Eye, EyeOff } from "lucide-react";
import { loginSchema, type LoginFormData } from "@/lib/validations/auth";
import { loginAction } from '@/app/actions/auth';
import { getPortalRedirectUrl } from '@/lib/redirect';
import { appConfig } from '@/config/app.config';
import { Button } from "@/components/ui";
import { Input } from "@/components/ui";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui";

export function LoginForm() {
  const [showPassword, setShowPassword] = useState(false);
  const [isLoading, setIsLoading] = useState(false);

  const {
    register,
    handleSubmit,
    control,
    formState: { errors },
  } = useForm<LoginFormData>({
    resolver: zodResolver(loginSchema),
  });

  const onSubmit = async (data: LoginFormData) => {
    setIsLoading(true);
    try {
      const result = await loginAction(data);
      
      if (result.success && result.token && result.user) {
        // Store token and user data
        localStorage.setItem(appConfig.auth.tokenKey, result.token);
        localStorage.setItem('user', JSON.stringify(result.user));
        
        // Get redirect URL from query params
        const urlParams = new URLSearchParams(window.location.search);
        const redirectUrl = urlParams.get('redirect');
        
        // Redirect to appropriate portal
        const portalUrl = getPortalRedirectUrl(result.user.role, redirectUrl || undefined);
        window.location.href = portalUrl;
      } else {
        console.error("Login failed:", result.message);
        // You can add toast notification here
      }
    } catch (error) {
      console.error("Login error:", error);
    } finally {
      setIsLoading(false);
    }
  };

  return (
    <motion.form
      initial={{ opacity: 0, y: 20 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{ duration: 0.5, ease: "easeOut" }}
      onSubmit={handleSubmit(onSubmit)}
      className="space-y-6"
    >
      {/* Header */}
      <div className="flex flex-col items-center justify-center space-y-1 text-center">
        <h1 className="text-3xl font-black text-blue-600 uppercase tracking-wide">
          University Portal
        </h1>
        <p className="text-lg text-gray-700 font-medium">Sign In</p>
      </div>

      {/* Form Fields */}
      <div className="space-y-4">
        {/* Role Dropdown */}
        <div>
          <label htmlFor="role" className="sr-only">
            Select Role
          </label>
          <Controller
            control={control}
            name="role"
            render={({ field }) => (
              <Select onValueChange={field.onChange} value={field.value}>
                <SelectTrigger 
                  className="w-full"
                  aria-invalid={errors.role ? "true" : "false"}
                >
                  <SelectValue placeholder="Select Role" />
                </SelectTrigger>
                <SelectContent className="bg-white">
                  <SelectItem value="student">Student</SelectItem>
                  <SelectItem value="faculty">Faculty</SelectItem>
                  <SelectItem value="parent">Parent</SelectItem>
                  <SelectItem value="admin">Administrator</SelectItem>
                </SelectContent>
              </Select>
            )}
          />
          {errors.role && (
            <p className="mt-1.5 text-sm text-red-600 font-medium" role="alert">
              {errors.role.message}
            </p>
          )}
        </div>

        {/* User ID Input */}
        <div>
          <label htmlFor="userId" className="sr-only">
            User ID
          </label>
          <Input
            id="userId"
            type="text"
            placeholder="User ID"
            autoComplete="username"
            {...register("userId")}
            aria-invalid={errors.userId ? "true" : "false"}
          />
          {errors.userId && (
            <p className="mt-1.5 text-sm text-red-600 font-medium" role="alert">
              {errors.userId.message}
            </p>
          )}
        </div>

        {/* Password Input with Eye Toggle */}
        <div>
          <label htmlFor="password" className="sr-only">
            Password
          </label>
          <div className="relative">
            <Input
              id="password"
              type={showPassword ? "text" : "password"}
              placeholder="Password"
              autoComplete="current-password"
              {...register("password")}
              aria-invalid={errors.password ? "true" : "false"}
              className="pr-12"
            />
            <button
              type="button"
              onClick={() => setShowPassword(!showPassword)}
              className="absolute right-3 top-1/2 -translate-y-1/2 text-gray-400 hover:text-gray-600 transition-colors"
              aria-label={showPassword ? "Hide password" : "Show password"}
            >
              {showPassword ? (
                <EyeOff className="h-5 w-5" />
              ) : (
                <Eye className="h-5 w-5" />
              )}
            </button>
          </div>
          {errors.password && (
            <p className="mt-1.5 text-sm text-red-600 font-medium" role="alert">
              {errors.password.message}
            </p>
          )}
        </div>
      </div>

      {/* Forgot Password Link */}
      <div className="text-right">
        <a
          href="/forgot-password"
          className="text-sm text-blue-600 hover:text-blue-700 underline-offset-2 hover:underline font-medium"
        >
          Forgot Password?
        </a>
      </div>

      {/* Login Button */}
      <Button
        type="submit"
        size="lg"
        disabled={isLoading}
        className="w-full font-bold uppercase tracking-wide"
      >
        {isLoading ? (
          <span className="flex items-center justify-center gap-2">
            <svg
              className="animate-spin h-5 w-5"
              xmlns="http://www.w3.org/2000/svg"
              fill="none"
              viewBox="0 0 24 24"
            >
              <circle
                className="opacity-25"
                cx="12"
                cy="12"
                r="10"
                stroke="currentColor"
                strokeWidth="4"
              />
              <path
                className="opacity-75"
                fill="currentColor"
                d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"
              />
            </svg>
            Logging in...
          </span>
        ) : (
          "Login"
        )}
      </Button>

      {/* Registration Link */}
      <div className="text-center pt-2">
        <p className="text-sm text-gray-600">
          Don't have an account?{" "}
          <a
            href="/register"
            className="text-blue-600 font-semibold hover:text-blue-700 underline-offset-2 hover:underline"
          >
            Register here
          </a>
        </p>
      </div>
    </motion.form>
  );
}