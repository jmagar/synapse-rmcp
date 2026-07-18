import { Button } from "@/components/ui/button";

export function ActionButton({
  onClick,
  label,
  loading = false,
  disabled = false,
}: {
  onClick: () => void;
  label: string;
  loading?: boolean;
  disabled?: boolean;
}) {
  return (
    <Button type="button" onClick={onClick} variant="aurora" disabled={disabled || loading}>
      {loading ? `${label}…` : label}
    </Button>
  );
}
