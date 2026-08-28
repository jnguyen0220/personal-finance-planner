"use client";

import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { api, formatDateTime, type LogLevel } from "@/lib/api";

const LEVEL_BADGE: Record<LogLevel, string> = {
  error: "bg-red-100 text-red-700",
  warning: "bg-amber-100 text-amber-700",
  info: "bg-sky-100 text-sky-700",
};

/// Recent application errors and warnings, for troubleshooting unattended
/// background failures (daily jobs, Gmail polling, SMS sending).
export function LogsAdmin() {
  const queryClient = useQueryClient();
  const {
    data: logs = [],
    isLoading,
    error,
  } = useQuery({ queryKey: ["logs"], queryFn: api.logs });

  const clear = useMutation({
    mutationFn: api.clearLogs,
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ["logs"] }),
  });

  return (
    <section className="card p-5">
      <div className="mb-3 flex items-start justify-between gap-3">
        <div>
          <h2 className="font-semibold tracking-tight">Logs</h2>
          <p className="mt-0.5 text-sm text-[var(--muted)]">
            Recent errors and warnings from background jobs. Use these to
            troubleshoot failures in the daily schedule, Gmail polling, or
            reminders.
          </p>
        </div>
        <div className="flex items-center gap-2">
          <button
            type="button"
            onClick={() => queryClient.invalidateQueries({ queryKey: ["logs"] })}
            className="link-action"
          >
            Refresh
          </button>
          {logs.length > 0 && (
            <button
              type="button"
              onClick={() => clear.mutate()}
              disabled={clear.isPending}
              className="link-muted disabled:opacity-50"
            >
              {clear.isPending ? "Clearing…" : "Clear all"}
            </button>
          )}
        </div>
      </div>

      {isLoading ? (
        <p className="text-sm text-[var(--muted)]">Loading…</p>
      ) : error ? (
        <p className="text-sm text-red-600">{(error as Error).message}</p>
      ) : logs.length === 0 ? (
        <p className="py-8 text-center text-sm text-[var(--muted)]">
          No errors logged. Everything is running cleanly.
        </p>
      ) : (
        <ul className="divide-y divide-[var(--border)]">
          {logs.map((log) => (
            <li key={log.id} className="py-3">
              <div className="flex items-center gap-2">
                <span className={`badge ${LEVEL_BADGE[log.level] ?? LEVEL_BADGE.info}`}>
                  {log.level}
                </span>
                {log.source && (
                  <span className="font-mono text-xs text-[var(--muted)]">{log.source}</span>
                )}
                <span className="ml-auto text-xs text-[var(--muted)]">
                  {formatDateTime(log.created_at)}
                </span>
              </div>
              <p className="mt-1 whitespace-pre-wrap break-words text-sm">{log.message}</p>
            </li>
          ))}
        </ul>
      )}
    </section>
  );
}
