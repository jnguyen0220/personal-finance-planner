type MoneyInputProps = Omit<React.InputHTMLAttributes<HTMLInputElement>, "type"> & {
  className?: string;
};

/// A number input with a leading "$" so currency fields read as money.
export function MoneyInput({ className = "input", ...props }: MoneyInputProps) {
  return (
    <div className="relative">
      <span className="pointer-events-none absolute left-3 top-1/2 -translate-y-1/2 text-sm text-[var(--muted)]">
        $
      </span>
      <input type="number" step="0.01" min="0" className={`${className} pl-6`} {...props} />
    </div>
  );
}
