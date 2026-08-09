import type { ButtonHTMLAttributes, ReactNode } from "react";
import "./Button.css";

interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: "primary" | "secondary" | "ghost";
  size?: "sm" | "md";
  children: ReactNode;
}

export function Button({
  variant = "secondary",
  size = "md",
  children,
  className = "",
  ...props
}: ButtonProps) {
  return (
    <button
      className={`btn btn--${variant} btn--${size} ${className}`.trim()}
      {...props}
    >
      {children}
    </button>
  );
}
