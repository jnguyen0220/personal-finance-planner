import Link from "next/link";
import InboxReview from "@/features/inbox/InboxReview";

export const metadata = {
  title: "Invoice review",
};

export default function InboxPage() {
  return (
    <main className="mx-auto max-w-5xl px-6 py-8">
      <div className="mb-6">
        <div className="flex items-center gap-2 text-sm text-[var(--muted)]">
          <Link href="/" className="transition hover:text-[var(--foreground)]">
            Portfolio
          </Link>
          <span>/</span>
          <span>Invoice review</span>
        </div>
        <h1 className="mt-1 text-2xl font-bold tracking-tight">Invoice review</h1>
        <p className="mt-1 text-sm text-[var(--muted)]">
          Invoices emailed to your Gmail inbox. Confirm the amount, pick a property and
          category, then file each one as an expense with the attachment as its receipt.
        </p>
      </div>
      <InboxReview />
    </main>
  );
}
