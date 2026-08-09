import type { AuthErrorCode, FieldErrors, SignUpCredentials } from "./types";

const EMAIL_PATTERN = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;

export function validateEmail(email: string): AuthErrorCode | null {
  const value = email.trim();
  if (!value) return "invalid_email";
  if (!EMAIL_PATTERN.test(value)) return "invalid_email";
  return null;
}

export function validateDisplayName(name: string): AuthErrorCode | null {
  const value = name.trim();
  if (value.length < 2) return "invalid_name";
  if (value.length > 60) return "invalid_name";
  return null;
}

export function validateLoginPassword(password: string): AuthErrorCode | null {
  if (!password) return "invalid_credentials";
  if (password.length < 6) return "invalid_credentials";
  return null;
}

export function validateRegisterPassword(password: string): AuthErrorCode | null {
  if (password.length < 8) return "weak_password";
  return null;
}

export function validateLoginForm(input: {
  email: string;
  password: string;
}): FieldErrors | null {
  const errors: FieldErrors = {};
  const emailError = validateEmail(input.email);
  const passwordError = validateLoginPassword(input.password);

  if (emailError) errors.email = emailError;
  if (passwordError) errors.password = passwordError;

  return Object.keys(errors).length > 0 ? errors : null;
}

export function validateRegisterForm(
  input: SignUpCredentials,
): FieldErrors | null {
  const errors: FieldErrors = {};
  const nameError = validateDisplayName(input.displayName);
  const emailError = validateEmail(input.email);
  const passwordError = validateRegisterPassword(input.password);

  if (nameError) errors.displayName = nameError;
  if (emailError) errors.email = emailError;
  if (passwordError) errors.password = passwordError;

  if (
    input.confirmPassword !== undefined &&
    input.confirmPassword !== input.password
  ) {
    errors.confirmPassword = "password_mismatch";
  }

  return Object.keys(errors).length > 0 ? errors : null;
}

export function validateForgotPasswordForm(email: string): FieldErrors | null {
  const emailError = validateEmail(email);
  return emailError ? { email: emailError } : null;
}
