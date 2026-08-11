export function Field({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <label className="flex flex-col">
      <span className="label">{label}</span>
      {children}
    </label>
  );
}
