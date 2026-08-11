#!/usr/bin/env python3
"""Truncate the database and seed realistic data exercising every feature.

Generates 5 rental properties, each with a previous and a current tenant
(10 tenants total) across ~5 years of history (2021-2026). Every current
tenant embodies a distinct behaviour so the whole app can be exercised:

  * 12 Oak Street   - GOOD payer: rent on time, nothing owed.
  * 48 Maple Avenue - LATE payer: pays rent late many months and is charged a
                      separate "late fee" (new income category / lease late_fee).
  * 7 Birch Lane    - DELINQUENT: misses several months, still owes a balance.
                      Insurance also lapses after 2024 (expired policy).
  * 230 Elm Court   - CARRY-OVER then CATCH-UP: falls behind in 2024, repays
                      the arrears in 2025, back to zero owed today.
  * 91 Cedar Road   - OUT-OF-POCKET: tenant pays for a repair and yard work from
                      their own pocket (borne_by = tenant), credited against rent.

Also seeds mortgages, property tax, utilities income, landlord repairs and
yearly insurance policies (active / expiring / expired) plus the leases'
new payment_date and late_fee fields.

Usage: python3 scripts/seed.py

Clears properties, tenants, leases, transactions and insurance_policies. The
attachments table (uploaded files) is left untouched.
"""

import os
import random
import sqlite3
import uuid
from datetime import date, datetime, timedelta

DB_PATH = os.path.join(os.path.dirname(__file__), "..", "backend", "data", "app.db")
TODAY = date(2026, 8, 11)
WINDOW_START = date(2021, 1, 1)

random.seed(7)

# The current tenant's lease starts here; the previous tenant covers before it,
# so no two tenants collect rent in the same calendar year (keeps owed-rent math clean).
CURRENT_START = date(2024, 1, 1)
CURRENT_END = date(2027, 6, 30)  # extends into the future -> still current
PREV_START = WINDOW_START
PREV_END = date(2023, 12, 31)

DUE_DAY = 5  # rent is due by the 5th; paying later incurs the lease late fee

CREATED = datetime.utcnow().replace(microsecond=0).isoformat() + "Z"


def new_id() -> str:
    return str(uuid.uuid4())


def iso(d: date) -> str:
    return d.isoformat()


def add_months(d: date, months: int) -> date:
    m = d.month - 1 + months
    y = d.year + m // 12
    m = m % 12 + 1
    leap = y % 4 == 0 and (y % 100 != 0 or y % 400 == 0)
    day = min(d.day, [31, 29 if leap else 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31][m - 1])
    return date(y, m, day)


def month_starts(start: date, end: date):
    """First-of-month dates from start's month through end's month inclusive."""
    m = date(start.year, start.month, 1)
    last = date(end.year, end.month, 1)
    while m <= last:
        yield m
        m = add_months(m, 1)


SCHEMA = """
CREATE TABLE IF NOT EXISTS properties (
    id TEXT PRIMARY KEY, name TEXT NOT NULL, address TEXT NOT NULL DEFAULT '',
    city TEXT NOT NULL DEFAULT '', state TEXT NOT NULL DEFAULT '', zip TEXT NOT NULL DEFAULT '',
    kind TEXT NOT NULL DEFAULT 'rental', purchase_date TEXT,
    notes TEXT NOT NULL DEFAULT '', created_at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS tenants (
    id TEXT PRIMARY KEY, property_id TEXT NOT NULL, name TEXT NOT NULL,
    email TEXT NOT NULL DEFAULT '', phone TEXT NOT NULL DEFAULT '',
    is_current INTEGER NOT NULL DEFAULT 0, notes TEXT NOT NULL DEFAULT '',
    driver_license_id TEXT, created_at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS leases (
    id TEXT PRIMARY KEY, tenant_id TEXT NOT NULL, monthly_rent REAL NOT NULL DEFAULT 0,
    start_date TEXT, end_date TEXT, payment_date TEXT, late_fee REAL NOT NULL DEFAULT 0,
    notes TEXT NOT NULL DEFAULT '', created_at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS transactions (
    id TEXT PRIMARY KEY, property_id TEXT NOT NULL, kind TEXT NOT NULL,
    category TEXT NOT NULL DEFAULT 'other', amount REAL NOT NULL, date TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '', tenant_name TEXT NOT NULL DEFAULT '',
    borne_by TEXT NOT NULL DEFAULT 'landlord', receipt_id TEXT, created_at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS insurance_policies (
    id TEXT PRIMARY KEY, property_id TEXT NOT NULL, provider TEXT NOT NULL,
    policy_number TEXT NOT NULL DEFAULT '', premium REAL NOT NULL DEFAULT 0,
    start_date TEXT, expiry_date TEXT NOT NULL, notes TEXT NOT NULL DEFAULT '',
    created_at TEXT NOT NULL
);
"""


def ensure_schema(db: sqlite3.Connection) -> None:
    db.executescript(SCHEMA)
    # Add columns that older databases may be missing.
    wanted = {
        "properties": [("state", "TEXT NOT NULL DEFAULT ''")],
        "leases": [("payment_date", "TEXT"), ("late_fee", "REAL NOT NULL DEFAULT 0")],
        "tenants": [("is_current", "INTEGER NOT NULL DEFAULT 0"), ("driver_license_id", "TEXT")],
        "transactions": [("tenant_name", "TEXT NOT NULL DEFAULT ''"),
                         ("borne_by", "TEXT NOT NULL DEFAULT 'landlord'")],
    }
    for table, cols in wanted.items():
        have = {r[1] for r in db.execute(f"PRAGMA table_info({table})")}
        for name, decl in cols:
            if name not in have:
                db.execute(f"ALTER TABLE {table} ADD COLUMN {name} {decl}")


PROPERTIES = [
    dict(name="12 Oak Street", address="12 Oak Street", city="Springfield", state="IL", zip="62704",
         purchase_date="2019-06-10", notes="Two-bed single family",
         base_rent=1500, mortgage=920, tax=3600, has_mortgage=True,
         profile="good", renewal_month=1, insurance="normal"),
    dict(name="48 Maple Avenue", address="48 Maple Avenue", city="Riverside", state="CA", zip="92501",
         purchase_date="2020-03-22", notes="Three-bed townhouse",
         base_rent=1800, mortgage=1150, tax=4200, has_mortgage=True,
         profile="late", renewal_month=9, insurance="expiring"),
    dict(name="7 Birch Lane", address="7 Birch Lane", city="Lakeside", state="CA", zip="92040",
         purchase_date="2018-11-05", notes="One-bed condo",
         base_rent=1250, mortgage=780, tax=3000, has_mortgage=True,
         profile="outstanding", renewal_month=1, insurance="lapsed"),
    dict(name="230 Elm Court", address="230 Elm Court", city="Fairview", state="OR", zip="97024",
         purchase_date="2020-11-15", notes="Four-bed family home",
         base_rent=2200, mortgage=1400, tax=5200, has_mortgage=True,
         profile="catchup", renewal_month=3, insurance="normal"),
    dict(name="91 Cedar Road", address="91 Cedar Road", city="Hillcrest", state="MN", zip="55305",
         purchase_date="2017-08-30", notes="Duplex, owned outright",
         base_rent=1650, mortgage=0, tax=3900, has_mortgage=False,
         profile="out_of_pocket", renewal_month=6, insurance="normal"),
]

# Two names per property: (previous tenant, current tenant).
TENANT_PAIRS = [
    ("Emily Carter", "James Nguyen"),
    ("Sofia Rossi", "Liam O'Brien"),
    ("Aisha Khan", "Noah Bennett"),
    ("Maria Gonzalez", "Ethan Walker"),
    ("Priya Sharma", "Lucas Meyer"),
]

INSURERS = ["Statewide Mutual", "Harbor Insurance", "Cedar Point Assurance",
            "Northgate Cover", "Pinnacle Underwriters"]

LANDLORD_REPAIRS = [
    ("Roof patch", 400, 2200), ("HVAC servicing", 150, 700),
    ("Appliance replacement", 350, 1400), ("Plumbing repair", 180, 850),
    ("Electrical fix", 200, 950),
]

counts = dict(properties=0, tenants=0, leases=0, transactions=0, insurance=0,
              late_fees=0, tenant_paid=0, missed=0)


def email_for(name: str) -> str:
    return name.lower().replace(" ", ".").replace("'", "") + "@example.com"


def phone() -> str:
    return f"555-0{random.randint(100, 999)}-{random.randint(1000, 9999)}"


def add_tx(db, prop_id, kind, category, amount, d, desc,
           tenant_name="", borne_by="landlord"):
    db.execute(
        "INSERT INTO transactions (id, property_id, kind, category, amount, date, "
        "description, tenant_name, borne_by, receipt_id, created_at) "
        "VALUES (?,?,?,?,?,?,?,?,?,?,?)",
        (new_id(), prop_id, kind, category, float(amount), iso(d), desc,
         tenant_name, borne_by, None, CREATED),
    )
    counts["transactions"] += 1


def add_lease(db, tenant_id, rent, start, end, late_fee):
    payment_date = iso(date(start.year, start.month, DUE_DAY))
    db.execute(
        "INSERT INTO leases (id, tenant_id, monthly_rent, start_date, end_date, "
        "payment_date, late_fee, notes, created_at) VALUES (?,?,?,?,?,?,?,?,?)",
        (new_id(), tenant_id, float(rent), iso(start), iso(end),
         payment_date, float(late_fee), f"Rent due by the {DUE_DAY}th.", CREATED),
    )
    counts["leases"] += 1


def add_tenant(db, prop_id, name, is_current, notes):
    tid = new_id()
    db.execute(
        "INSERT INTO tenants (id, property_id, name, email, phone, is_current, "
        "notes, driver_license_id, created_at) VALUES (?,?,?,?,?,?,?,?,?)",
        (tid, prop_id, name, email_for(name), phone(), 1 if is_current else 0,
         notes, None, CREATED),
    )
    counts["tenants"] += 1
    return tid


def active_name(spans, d):
    ds = iso(d)
    for start, end, name in spans:
        if start <= ds and (end is None or ds <= end):
            return name
    return ""


def emit_ontime_rent(db, prop_id, name, rent, start, end):
    """Full rent paid on time (by the due day) every month up to today."""
    for m in month_starts(start, min(end, TODAY)):
        d = date(m.year, m.month, random.randint(1, DUE_DAY))
        if start <= d <= TODAY:
            add_tx(db, prop_id, "income", "rent", rent, d, f"Monthly rent - {name}", name)


def emit_current_tenant(db, prop_id, name, rent, late_fee, profile):
    """Rent history for the current tenant, shaped by their payer profile."""
    months = list(month_starts(CURRENT_START, min(CURRENT_END, TODAY)))

    if profile == "good":
        emit_ontime_rent(db, prop_id, name, rent, CURRENT_START, CURRENT_END)

    elif profile == "late":
        # Pays full rent but often after the due date, incurring the late fee.
        for i, m in enumerate(months):
            late = i > 0 and random.random() < 0.45
            pay_day = random.randint(9, 24) if late else random.randint(1, DUE_DAY)
            d = min(date(m.year, m.month, pay_day), TODAY)
            if d < CURRENT_START:
                continue
            add_tx(db, prop_id, "income", "rent", rent, d, f"Monthly rent - {name}", name)
            if late:
                add_tx(db, prop_id, "income", "late fee", late_fee, d,
                       f"Late payment fee - {name}", name)
                counts["late_fees"] += 1

    elif profile == "outstanding":
        # Misses several months outright and never repays -> owes a balance.
        missed = {(2025, 6), (2025, 7), (2026, 3), (2026, 4), (2026, 5)}
        for m in months:
            if (m.year, m.month) in missed:
                counts["missed"] += 1
                continue
            d = min(date(m.year, m.month, random.randint(1, DUE_DAY)), TODAY)
            add_tx(db, prop_id, "income", "rent", rent, d, f"Monthly rent - {name}", name)

    elif profile == "catchup":
        # Falls behind by 3 months in 2024, repays the arrears during 2025.
        arrears = {(2024, 3), (2024, 4), (2024, 5)}
        for m in months:
            if (m.year, m.month) in arrears:
                counts["missed"] += 1
                continue
            d = min(date(m.year, m.month, random.randint(1, DUE_DAY)), TODAY)
            add_tx(db, prop_id, "income", "rent", rent, d, f"Monthly rent - {name}", name)
        for i in range(len(arrears)):
            d = date(2025, 4 + i * 2, 20)  # back-payments spread across 2025
            add_tx(db, prop_id, "income", "rent", rent, d,
                   f"Back-payment (arrears) - {name}", name)

    elif profile == "out_of_pocket":
        # Pays rent on time, except two months covered by tenant-funded work.
        credit_repair = (2025, 4)
        credit_yard = (2026, 5)
        for m in months:
            key = (m.year, m.month)
            if key == credit_repair:
                add_tx(db, prop_id, "expense", "repair", rent, date(m.year, m.month, 12),
                       f"Water heater replacement (paid by tenant) - {name}",
                       name, borne_by="tenant")
                counts["tenant_paid"] += 1
                continue
            if key == credit_yard:
                add_tx(db, prop_id, "expense", "other", rent, date(m.year, m.month, 8),
                       f"Yard landscaping (paid by tenant) - {name}",
                       name, borne_by="tenant")
                counts["tenant_paid"] += 1
                continue
            d = min(date(m.year, m.month, random.randint(1, DUE_DAY)), TODAY)
            add_tx(db, prop_id, "income", "rent", rent, d, f"Monthly rent - {name}", name)


def emit_landlord_costs(db, prop_id, p, tenant_spans):
    """Mortgage, tax, utilities income and landlord repairs across the window."""
    for m in month_starts(WINDOW_START, TODAY):
        if p["has_mortgage"]:
            add_tx(db, prop_id, "expense", "mortgage", p["mortgage"],
                   date(m.year, m.month, 1), "Mortgage payment")
        if m.month == 4:  # annual property tax
            add_tx(db, prop_id, "expense", "tax", p["tax"],
                   date(m.year, 4, 15), f"Property tax {m.year}")
        if m.month in (1, 4, 7, 10):  # quarterly utilities billed to the tenant
            d = date(m.year, m.month, random.randint(8, 15))
            if d <= TODAY:
                add_tx(db, prop_id, "income", "utilities", random.randint(80, 180), d,
                       "Utilities reimbursement", active_name(tenant_spans, d))

    for yr in range(WINDOW_START.year, TODAY.year + 1):
        for _ in range(random.randint(1, 3)):
            d = date(yr, random.randint(1, 12), random.randint(1, 28))
            if WINDOW_START <= d <= TODAY:
                desc, lo, hi = random.choice(LANDLORD_REPAIRS)
                add_tx(db, prop_id, "expense", "repair", random.randint(lo, hi), d, desc)


def emit_insurance(db, prop_id, pi, p):
    premium = round((p["base_rent"] * 0.6) / 10) * 10
    for yr in range(WINDOW_START.year, TODAY.year + 1):
        if p["insurance"] == "lapsed" and yr > 2024:
            continue  # owner let coverage lapse
        start = date(yr, p["renewal_month"], 1)
        if start > TODAY:
            continue
        expiry = add_months(start, 12) - timedelta(days=1)
        db.execute(
            "INSERT INTO insurance_policies (id, property_id, provider, policy_number, "
            "premium, start_date, expiry_date, notes, created_at) VALUES (?,?,?,?,?,?,?,?,?)",
            (new_id(), prop_id, INSURERS[pi % len(INSURERS)],
             f"POL-{yr}-{1000 + pi * 7 + yr % 100}", float(premium),
             iso(start), iso(expiry), "Landlord property policy", CREATED),
        )
        counts["insurance"] += 1
        add_tx(db, prop_id, "expense", "insurance", premium, start,
               f"Insurance premium {yr}")


def main() -> None:
    db = sqlite3.connect(os.path.abspath(DB_PATH))
    db.execute("PRAGMA foreign_keys = ON")
    ensure_schema(db)

    # Reset domain tables (uploaded attachments left untouched).
    for table in ("transactions", "insurance_policies", "leases", "tenants", "properties"):
        db.execute(f"DELETE FROM {table}")

    for pi, p in enumerate(PROPERTIES):
        prop_id = new_id()
        db.execute(
            "INSERT INTO properties (id, name, address, city, state, zip, kind, purchase_date, "
            "notes, created_at) VALUES (?,?,?,?,?,?,?,?,?,?)",
            (prop_id, p["name"], p["address"], p["city"], p["state"], p["zip"], "rental",
             p["purchase_date"], p["notes"], CREATED),
        )
        counts["properties"] += 1

        prev_name, cur_name = TENANT_PAIRS[pi]
        prev_rent = round(p["base_rent"] / 25) * 25
        cur_rent = round((p["base_rent"] * 1.08) / 25) * 25
        late_fee = round((cur_rent * 0.05) / 5) * 5

        # Previous tenant: clean, on-time history 2021-2023.
        prev_id = add_tenant(db, prop_id, prev_name, False, "Moved out, lease ended.")
        add_lease(db, prev_id, prev_rent, PREV_START, PREV_END, round((prev_rent * 0.05) / 5) * 5)
        emit_ontime_rent(db, prop_id, prev_name, prev_rent, PREV_START, PREV_END)

        # Current tenant: behaviour depends on the property's profile.
        cur_notes = {
            "good": "Reliable, always pays on time.",
            "late": "Frequently pays late; late fees applied.",
            "outstanding": "Behind on rent; balance outstanding.",
            "catchup": "Fell behind in 2024, has since caught up.",
            "out_of_pocket": "Handles some repairs/yard work at own cost.",
        }[p["profile"]]
        cur_id = add_tenant(db, prop_id, cur_name, True, cur_notes)
        add_lease(db, cur_id, cur_rent, CURRENT_START, CURRENT_END, late_fee)
        emit_current_tenant(db, prop_id, cur_name, cur_rent, late_fee, p["profile"])

        spans = [
            (iso(PREV_START), iso(PREV_END), prev_name),
            (iso(CURRENT_START), None, cur_name),
        ]
        emit_landlord_costs(db, prop_id, p, spans)
        emit_insurance(db, prop_id, pi, p)

    db.commit()
    db.close()

    print("Seed complete:")
    for key in ("properties", "tenants", "leases", "insurance", "transactions",
                "late_fees", "tenant_paid", "missed"):
        print(f"  {key:12} {counts[key]}")


if __name__ == "__main__":
    main()
