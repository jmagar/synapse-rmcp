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
  type?: "text" | "number" | "checkbox" | "string-list" | "select";
  options?: readonly string[];
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
  options,
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

  if (type === "select") {
    return (
      <select
        id={id}
        value={value}
        required={required}
        onChange={(event) => onChange(event.target.value)}
        className="h-10 w-full rounded-md border px-3 text-sm"
        style={{ background: "var(--aurora-control-surface)" }}
      >
        <option value="">Select a target</option>
        {options?.map((option) => (
          <option key={option} value={option}>
            {option}
          </option>
        ))}
      </select>
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
