"use client";

/// A reusable on/off toggle switch. Renders only the control; callers supply
/// their own surrounding <label> and text so wording stays contextual.
export function Switch({
  checked,
  onChange,
  disabled = false,
  size = "md",
}: {
  checked: boolean;
  onChange: (checked: boolean) => void;
  disabled?: boolean;
  size?: "sm" | "md";
}) {
  const s =
    size === "sm"
      ? { track: "h-5 w-9", knob: "h-4 w-4 peer-checked:translate-x-4" }
      : { track: "h-6 w-11", knob: "h-5 w-5 peer-checked:translate-x-5" };
  return (
    <span className={`relative inline-flex ${s.track} shrink-0 items-center`}>
      <input
        type="checkbox"
        className="peer sr-only"
        checked={checked}
        disabled={disabled}
        onChange={(e) => onChange(e.target.checked)}
      />
      <span className="absolute inset-0 rounded-full bg-[var(--border)] transition peer-checked:bg-indigo-600" />
      <span
        className={`absolute left-0.5 rounded-full bg-white shadow transition ${s.knob}`}
      />
    </span>
  );
}
