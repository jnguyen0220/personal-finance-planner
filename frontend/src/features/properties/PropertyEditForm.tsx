"use client";

import { useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { api, statesQueryOptions, type Property, type PropertyKind } from "@/lib/api";
import { Field } from "@/components/ui/Field";
import { Switch } from "@/components/ui/Switch";
import { Modal } from "@/components/ui/Modal";

export function PropertyEditForm({
  property,
  onClose,
  onSaved,
}: {
  property: Property;
  onClose: () => void;
  onSaved: () => Promise<void>;
}) {
  const [name, setName] = useState(property.name);
  const [kind, setKind] = useState<PropertyKind>(property.kind);
  const [address, setAddress] = useState(property.address);
  const [city, setCity] = useState(property.city);
  const [stateField, setStateField] = useState(property.state);
  const [zip, setZip] = useState(property.zip);
  const [purchaseDate, setPurchaseDate] = useState(property.purchase_date ?? "");
  const [notes, setNotes] = useState(property.notes);
  const [hoaName, setHoaName] = useState(property.hoa_name);
  const [hoaPhone, setHoaPhone] = useState(property.hoa_phone);
  const [hoaEmail, setHoaEmail] = useState(property.hoa_email);
  const [hoaWebpage, setHoaWebpage] = useState(property.hoa_webpage);
  const [remindersEnabled, setRemindersEnabled] = useState(property.reminders_enabled);
  const [saving, setSaving] = useState(false);
  const [err, setErr] = useState<string | null>(null);
  const [tab, setTab] = useState<"details" | "hoa">("details");

  const { data: states = [] } = useQuery(statesQueryOptions);

  async function submit(e: React.FormEvent) {
    e.preventDefault();
    setErr(null);
    setSaving(true);
    try {
      await api.updateProperty(property.id, {
        name,
        kind,
        address,
        city,
        state: stateField,
        zip,
        purchase_date: purchaseDate || null,
        notes,
        hoa_name: hoaName,
        hoa_phone: hoaPhone,
        hoa_email: hoaEmail,
        hoa_webpage: hoaWebpage,
        reminders_enabled: remindersEnabled,
      });
      await onSaved();
    } catch (e) {
      setErr((e as Error).message);
    } finally {
      setSaving(false);
    }
  }

  return (
    <Modal title="Edit property" onClose={onClose} onSubmit={submit} error={err} saving={saving}>
      <div className="mb-4 flex gap-1 border-b border-[var(--border)]">
        <button
          type="button"
          onClick={() => setTab("details")}
          className={`tab ${tab === "details" ? "tab-active" : ""}`}
        >
          Details
        </button>
        <button
          type="button"
          onClick={() => setTab("hoa")}
          className={`tab ${tab === "hoa" ? "tab-active" : ""}`}
        >
          HOA
        </button>
      </div>

      <div className={tab === "details" ? "" : "hidden"}>
        <div className="grid grid-cols-2 gap-3">
          <Field label="Name">
            <input className="input" value={name} onChange={(e) => setName(e.target.value)} required />
          </Field>
          <Field label="Type">
            <select className="input" value={kind} onChange={(e) => setKind(e.target.value as PropertyKind)}>
              <option value="rental">Rental</option>
              <option value="personal">Personal</option>
            </select>
          </Field>
          <Field label="Address">
            <input className="input" value={address} onChange={(e) => setAddress(e.target.value)} />
          </Field>
          <Field label="City">
            <input className="input" value={city} onChange={(e) => setCity(e.target.value)} />
          </Field>
          <Field label="State">
            <select className="input" value={stateField} onChange={(e) => setStateField(e.target.value)}>
              <option value="">—</option>
              {states.map((s) => (
                <option key={s.code} value={s.code}>
                  {s.name}
                </option>
              ))}
            </select>
          </Field>
          <Field label="Zip">
            <input className="input" value={zip} onChange={(e) => setZip(e.target.value)} />
          </Field>
          <Field label="Purchase date">
            <input type="date" className="input" value={purchaseDate} onChange={(e) => setPurchaseDate(e.target.value)} />
          </Field>
        </div>
        <div className="mt-3">
          <Field label="Notes">
            <textarea className="input" rows={3} value={notes} onChange={(e) => setNotes(e.target.value)} />
          </Field>
        </div>
        {kind === "rental" && (
          <div className="mt-3">
            <Field label="Automated reminders">
              <label className="flex h-[38px] cursor-pointer items-center gap-3">
                <Switch checked={remindersEnabled} onChange={setRemindersEnabled} />
                <span className="text-sm text-[var(--muted)]">
                  {remindersEnabled
                    ? "Current tenant receives rent & lease reminders"
                    : "Paused — no automated texts"}
                </span>
              </label>
            </Field>
          </div>
        )}
      </div>

      <div className={tab === "hoa" ? "" : "hidden"}>
        <p className="mb-2 text-sm text-[var(--muted)]">
          Contact details tenants can use to reach the homeowners association.
        </p>
        <div className="grid grid-cols-2 gap-3">
          <Field label="HOA name">
            <input className="input" value={hoaName} onChange={(e) => setHoaName(e.target.value)} />
          </Field>
          <Field label="HOA phone">
            <input className="input" value={hoaPhone} onChange={(e) => setHoaPhone(e.target.value)} />
          </Field>
          <Field label="HOA email">
            <input className="input" value={hoaEmail} onChange={(e) => setHoaEmail(e.target.value)} />
          </Field>
          <Field label="HOA webpage">
            <input
              className="input"
              placeholder="https://…"
              value={hoaWebpage}
              onChange={(e) => setHoaWebpage(e.target.value)}
            />
          </Field>
        </div>
      </div>
    </Modal>
  );
}
