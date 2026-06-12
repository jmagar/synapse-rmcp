/**
 * ParamInput — a styled text input for the tool runner form.
 *
 * Uses the shared Aurora-compatible Input wrapper so focus and disabled states
 * stay aligned with the rest of the UI surface.
 */

"use client";

import { Input } from "@/components/ui/input";

interface ParamInputProps {
  id: string;
  type?: "text" | "number" | "checkbox" | "string-list";
  placeholder?: string;
  value: string;
  onChange: (value: string) => void;
  required?: boolean;
}

export function ParamInput({
  id,
  type = "text",
  placeholder,
  value,
  onChange,
  required,
}: ParamInputProps) {
  if (type === "checkbox") {
    return (
      <Input
        id={id}
        type="checkbox"
        checked={value === "true"}
        required={required}
        onChange={(e) => onChange(e.target.checked ? "true" : "")}
        className="h-4 w-4"
      />
    );
  }

  return (
    <Input
      id={id}
      type={type === "number" ? "number" : "text"}
      placeholder={placeholder}
      value={value}
      required={required}
      onChange={(e) => onChange(e.target.value)}
    />
  );
}
