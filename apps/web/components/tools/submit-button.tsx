/**
 * SubmitButton — a styled form submit button for the tool runner.
 *
 * Uses the shared Aurora-compatible Button wrapper for consistent states.
 */

"use client";

import { Button } from "@/components/ui/button";

interface SubmitButtonProps {
  loading: boolean;
  label?: string;
  loadingLabel?: string;
  disabled?: boolean;
}

export function SubmitButton({
  loading,
  label = "Run Action",
  loadingLabel = "Running…",
  disabled = false,
}: SubmitButtonProps) {
  return (
    <Button type="submit" disabled={loading || disabled} variant={loading ? "neutral" : "aurora"}>
      {loading ? loadingLabel : label}
    </Button>
  );
}
