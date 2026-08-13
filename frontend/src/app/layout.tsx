import type { Metadata } from "next";
import { Inter } from "next/font/google";
import Link from "next/link";
import Notifications from "@/components/Notifications";
import QueryProvider from "@/components/QueryProvider";
import "./globals.css";

const inter = Inter({ subsets: ["latin"], variable: "--font-inter" });

export const metadata: Metadata = {
  title: "Property Tracker",
  description: "Track income, expenses, tenants and insurance across your properties.",
};

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  return (
    <html lang="en">
      <body className={`${inter.variable} min-h-screen font-sans`}>
        <QueryProvider>
          <header className="sticky top-0 z-10 border-b border-[var(--border)] bg-[var(--surface)]/85 backdrop-blur-md print:hidden">
            <div className="mx-auto flex max-w-5xl items-center justify-between px-6 py-3">
              <Link href="/" className="group flex items-center gap-2.5">
                <span className="flex h-8 w-8 items-center justify-center rounded-lg bg-gradient-to-br from-indigo-500 to-indigo-700 text-sm font-bold text-white shadow-sm ring-1 ring-inset ring-white/10">
                  PT
                </span>
                <span className="text-base font-semibold tracking-tight transition group-hover:text-indigo-600">
                  Property Tracker
                </span>
              </Link>
              <div className="flex items-center gap-2 sm:gap-3">
                <Link
                  href="/tax"
                  className="flex items-center gap-1.5 rounded-lg px-3 py-1.5 text-sm font-medium text-[var(--muted)] transition hover:bg-[var(--surface-2)] hover:text-[var(--foreground)]"
                >
                  <TaxIcon />
                  Tax report
                </Link>
                <Link
                  href="/settings"
                  className="flex items-center gap-1.5 rounded-lg px-3 py-1.5 text-sm font-medium text-[var(--muted)] transition hover:bg-[var(--surface-2)] hover:text-[var(--foreground)]"
                >
                  <SettingsIcon />
                  Settings
                </Link>
                <Notifications />
              </div>
            </div>
          </header>
          {children}
        </QueryProvider>
      </body>
    </html>
  );
}

function TaxIcon() {
  return (
    <svg
      xmlns="http://www.w3.org/2000/svg"
      width="16"
      height="16"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
    >
      <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z" />
      <path d="M14 2v6h6" />
      <path d="M9 15l6-6" />
      <path d="M9.5 10.5h.01" />
      <path d="M14.5 15.5h.01" />
    </svg>
  );
}

function SettingsIcon() {
  return (
    <svg
      xmlns="http://www.w3.org/2000/svg"
      width="16"
      height="16"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
    >
      <circle cx="12" cy="12" r="3" />
      <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 1 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 1 1-2.83-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 1 1 2.83-2.83l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 1 1 2.83 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z" />
    </svg>
  );
}
