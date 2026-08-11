"use client";

/// Year picker shared by the portfolio and property views: an "All years"
/// option plus the supplied descending year list.
export function YearSelect({
  value,
  options,
  onChange,
  className = "input",
}: {
  value: number | "all";
  options: number[];
  onChange: (value: number | "all") => void;
  className?: string;
}) {
  return (
    <select
      className={className}
      value={String(value)}
      onChange={(e) => onChange(e.target.value === "all" ? "all" : Number(e.target.value))}
    >
      <option value="all">All years</option>
      {options.map((y) => (
        <option key={y} value={y}>
          {y}
        </option>
      ))}
    </select>
  );
}
