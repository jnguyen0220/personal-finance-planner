"use client";

import Link from "next/link";
import { useEffect, useRef, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { api, type NotificationSeverity } from "@/lib/api";
import { Switch } from "@/components/ui/Switch";

const SEVERITY_BADGE: Record<NotificationSeverity, string> = {
  error: "bg-red-100 text-red-700",
  warning: "bg-amber-100 text-amber-700",
  info: "bg-sky-100 text-sky-700",
};

export default function Notifications() {
  const [open, setOpen] = useState(false);
  const ref = useRef<HTMLDivElement>(null);
  const queryClient = useQueryClient();

  const { data: alerts = [] } = useQuery({
    queryKey: ["notifications"],
    queryFn: api.notifications,
  });

  const dismiss = useMutation({
    mutationFn: api.dismissNotification,
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ["notifications"] }),
  });

  const { data: settings } = useQuery({
    queryKey: ["settings"],
    queryFn: api.getSettings,
  });

  const toggleMessaging = useMutation({
    mutationFn: (enabled: boolean) => api.updateSettings({ messaging_enabled: enabled }),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ["settings"] }),
  });

  useEffect(() => {
    if (!open) return;
    function onDown(e: MouseEvent) {
      if (ref.current && !ref.current.contains(e.target as Node)) setOpen(false);
    }
    document.addEventListener("mousedown", onDown);
    return () => document.removeEventListener("mousedown", onDown);
  }, [open]);

  const count = alerts.length;

  return (
    <div className="relative" ref={ref}>
      <button
        onClick={() => setOpen((o) => !o)}
        aria-label="Notifications"
        className="relative flex h-9 w-9 items-center justify-center rounded-lg border border-[var(--border)] bg-[var(--surface)] text-[var(--muted)] transition hover:text-[var(--foreground)]"
      >
        <BellIcon />
        {count > 0 && (
          <span className="absolute -right-1 -top-1 flex h-4 min-w-4 items-center justify-center rounded-full bg-red-600 px-1 text-[10px] font-bold leading-none text-white">
            {count}
          </span>
        )}
      </button>

      {open && (
        <div className="absolute right-0 z-50 mt-2 w-80 overflow-hidden card p-0">
          <div className="flex items-center justify-between border-b border-[var(--border)] px-4 py-3">
            <span className="text-sm font-semibold">Notifications</span>
            {count > 0 && (
              <span className="badge bg-red-100 text-red-700">{count} action needed</span>
            )}
          </div>
          {count === 0 ? (
            <p className="px-4 py-8 text-center text-sm text-[var(--muted)]">
              You&apos;re all caught up.
            </p>
          ) : (
            <ul className="max-h-96 divide-y divide-[var(--border)] overflow-y-auto">
              {alerts.map((a) => {
                const title = a.link ? (
                  <Link
                    href={a.link}
                    onClick={() => setOpen(false)}
                    className="mt-1.5 block text-sm font-medium hover:text-indigo-600"
                  >
                    {a.title}
                  </Link>
                ) : (
                  <span className="mt-1.5 block text-sm font-medium">{a.title}</span>
                );
                return (
                  <li key={a.id} className="px-4 py-3">
                    <span className={`badge ${SEVERITY_BADGE[a.severity]}`}>{a.kind.replace(/_/g, " ")}</span>
                    {title}
                    {a.body && (
                      <p className="mt-0.5 text-xs text-[var(--muted)]">{a.body}</p>
                    )}
                    <button
                      onClick={() => dismiss.mutate(a.id)}
                      disabled={dismiss.isPending}
                      className="mt-2 link-action disabled:opacity-50"
                    >
                      Mark completed
                    </button>
                  </li>
                );
              })}
            </ul>
          )}
          <div className="flex items-center justify-between gap-3 border-t border-[var(--border)] px-4 py-3">
            <span className="text-xs text-[var(--muted)]">
              Automated tenant messages
            </span>
            <label className="flex cursor-pointer items-center gap-2">
              <span className="text-xs font-medium">
                {settings?.messaging_enabled === false ? "Off" : "On"}
              </span>
              <Switch
                size="sm"
                checked={settings?.messaging_enabled !== false}
                disabled={!settings || toggleMessaging.isPending}
                onChange={(checked) => toggleMessaging.mutate(checked)}
              />
            </label>
          </div>
        </div>
      )}
    </div>
  );
}

function BellIcon() {
  return (
    <svg
      xmlns="http://www.w3.org/2000/svg"
      width="18"
      height="18"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
    >
      <path d="M6 8a6 6 0 0 1 12 0c0 7 3 9 3 9H3s3-2 3-9" />
      <path d="M10.3 21a1.94 1.94 0 0 0 3.4 0" />
    </svg>
  );
}
