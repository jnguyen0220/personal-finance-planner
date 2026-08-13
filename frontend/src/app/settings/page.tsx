import { redirect } from "next/navigation";

// Settings has been folded into the Admin hub; keep the old path working.
export default function SettingsPage() {
  redirect("/admin");
}
