import type { InputHTMLAttributes } from "react";
import "./TextField.css";

interface TextFieldProps extends InputHTMLAttributes<HTMLInputElement> {
  label: string;
  error?: string;
  hint?: string;
}

export function TextField({
  label,
  error,
  hint,
  id,
  className = "",
  ...props
}: TextFieldProps) {
  const fieldId = id ?? props.name ?? label.toLowerCase().replace(/\s+/g, "-");

  return (
    <label className={`text-field ${className}`.trim()} htmlFor={fieldId}>
      <span className="text-field__label">{label}</span>
      <input
        id={fieldId}
        className={`text-field__input${error ? " is-invalid" : ""}`}
        {...props}
      />
      {error ? <span className="text-field__error">{error}</span> : null}
      {!error && hint ? <span className="text-field__hint">{hint}</span> : null}
    </label>
  );
}
