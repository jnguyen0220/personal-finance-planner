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
          <header className="sticky top-0 z-10 border-b border-[var(--border)] bg-[var(--surface)]/80 backdrop-blur print:hidden">
            <div className="mx-auto flex max-w-5xl items-center justify-between px-6 py-3">
              <Link href="/" className="flex items-center gap-2.5">
                <span className="flex h-8 w-8 items-center justify-center rounded-lg bg-indigo-600 text-sm font-bold text-white">
                  PT
                </span>
                <span className="text-base font-semibold tracking-tight">Property Tracker</span>
              </Link>
              <div className="flex items-center gap-4">
                <Link
                  href="/tax"
                  className="text-sm font-medium text-[var(--muted)] transition hover:text-[var(--foreground)]"
                >
                  Tax report
                </Link>
                <span className="hidden text-sm text-[var(--muted)] sm:block">
                  Portfolio dashboard
                </span>
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
