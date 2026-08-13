"use client";

import { useQuery } from "@tanstack/react-query";
import { statesQueryOptions } from "@/lib/api";

/// Read-only reference of the US states available for property addresses.
export function StatesList() {
  const { data: states = [] } = useQuery(statesQueryOptions);

  return (
    <section className="card p-5">
      <div className="mb-3">
        <h2 className="font-semibold tracking-tight">US states</h2>
        <p className="mt-0.5 text-sm text-[var(--muted)]">
          Reference list used for property addresses. {states.length} states available.
        </p>
      </div>
      <ul className="grid grid-cols-2 gap-x-4 gap-y-1 text-sm sm:grid-cols-3">
        {states.map((s) => (
          <li key={s.code} className="flex items-center gap-2">
            <span className="w-7 font-mono text-xs text-[var(--muted)]">{s.code}</span>
            <span>{s.name}</span>
          </li>
        ))}
      </ul>
    </section>
  );
}
