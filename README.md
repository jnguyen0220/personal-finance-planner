# pfp

A property tracker with a **Rust (axum + SQLite)** backend and a **Next.js (App Router, TypeScript, Tailwind)** frontend. Track income and expenses per property (rentals and your own home), tenants and rent, insurance policies with expiry reminders, and receipts attached to any transaction.

## Structure

```
pfp/
├── backend/    # Rust axum API + SQLite (port 8080)
├── frontend/   # Next.js app (port 3000)
└── scripts/    # helper run scripts
```

## Features

- **Properties** — rentals or personal homes, with per-property income/expense/net totals.
- **Transactions** — income (e.g. rent) and expenses (repairs, insurance, utilities, tax, mortgage), each optionally linked to a tenant and a receipt. On rentals, tenant-paid utilities are recorded as income rather than a landlord expense.
- **Tenants** — contact details, a current-tenant flag, and one or more leases (monthly rent, start/end).
- **Insurance** — provider, policy number, premium, expiry date; the dashboard highlights policies expiring soon or already expired.
- **Receipts** — upload images/PDF, stored on disk under `backend/data/uploads`, viewable per transaction.

## Data

SQLite database and uploaded receipts live in `backend/data/` (gitignored) and are created automatically on first run.

## Database schema

Defined in [backend/src/db.rs](backend/src/db.rs). Edit the tables here and mirror
the changes in `db.rs` (schema + migrations) and [backend/src/models.rs](backend/src/models.rs).

### `properties`

| Column          | Type | Notes                                                 |
| --------------- | ---- | ----------------------------------------------------- |
| `id`            | TEXT | primary key                                           |
| `name`          | TEXT | not null                                              |
| `address`       | TEXT | not null, default `''`                                |
| `city`          | TEXT | not null, default `''`                                |
| `state`         | TEXT | not null, default `''`                                |
| `zip`           | TEXT | not null, must be 5 digits when set, default `''`     |
| `kind`          | TEXT | not null, default `'rental'` (`rental` \| `personal`) |
| `purchase_date` | TEXT | nullable (ISO date)                                   |
| `notes`         | TEXT | not null, default `''`                                |
| `created_at`    | TEXT | not null (ISO date)                                   |

### `tenants`

| Column        | Type    | Notes                                                            |
| ------------- | ------- | ---------------------------------------------------------------- |
| `id`          | TEXT    | primary key                                                      |
| `property_id` | TEXT    | not null, FK → `properties(id)` on delete cascade                |
| `name`        | TEXT    | not null                                                         |
| `email`       | TEXT    | not null, default `''`                                           |
| `phone`       | TEXT    | not null, default `''`                                           |
| `is_current`  | INTEGER | not null, default `0` (boolean; one current tenant per property) |
| `notes`       | TEXT    | not null, default `''`                                           |
| `created_at`  | TEXT    | not null (ISO date)                                              |

### `leases`

| Column         | Type | Notes                                          |
| -------------- | ---- | ---------------------------------------------- |
| `id`           | TEXT | primary key                                    |
| `tenant_id`    | TEXT | not null, FK → `tenants(id)` on delete cascade |
| `monthly_rent` | REAL | not null, default `0`                          |
| `start_date`   | TEXT | nullable (ISO date)                            |
| `end_date`     | TEXT | nullable (ISO date)                            |
| `notes`        | TEXT | not null, default `''`                         |
| `created_at`   | TEXT | not null (ISO date)                            |

### `transactions`

| Column        | Type | Notes                                               |
| ------------- | ---- | --------------------------------------------------- |
| `id`          | TEXT | primary key                                         |
| `property_id` | TEXT | not null, FK → `properties(id)` on delete cascade   |
| `kind`        | TEXT | not null (`income` \| `expense`)                    |
| `category`    | TEXT | not null, default `'other'`                         |
| `amount`      | REAL | not null                                            |
| `date`        | TEXT | not null (ISO date)                                 |
| `description` | TEXT | not null, default `''`                              |
| `tenant_name` | TEXT | not null, default `''`                              |
| `receipt_id`  | TEXT | nullable, FK → `attachments(id)` on delete set null |
| `created_at`  | TEXT | not null (ISO date)                                 |

### `insurance_policies`

| Column          | Type | Notes                                             |
| --------------- | ---- | ------------------------------------------------- |
| `id`            | TEXT | primary key                                       |
| `property_id`   | TEXT | not null, FK → `properties(id)` on delete cascade |
| `provider`      | TEXT | not null                                          |
| `policy_number` | TEXT | not null, default `''`                            |
| `premium`       | REAL | not null, default `0`                             |
| `start_date`    | TEXT | nullable (ISO date)                               |
| `expiry_date`   | TEXT | not null (ISO date)                               |
| `notes`         | TEXT | not null, default `''`                            |
| `created_at`    | TEXT | not null (ISO date)                               |

### `attachments`

| Column          | Type    | Notes                        |
| --------------- | ------- | ---------------------------- |
| `id`            | TEXT    | primary key                  |
| `stored_name`   | TEXT    | not null (filename on disk)  |
| `original_name` | TEXT    | not null (uploaded filename) |
| `content_type`  | TEXT    | not null (MIME type)         |
| `size`          | INTEGER | not null (bytes)             |
| `uploaded_at`   | TEXT    | not null (ISO date)          |

## Prerequisites

Handled by the dev container: Rust toolchain and Node.js LTS.

## Backend (Rust, port 8080)

```bash
cd backend
cargo run
```

Endpoints:

- `GET /api/health` → `{ "status": "ok" }`
- `GET /api/categories?kind=rental|personal` → categories resolved for that property kind (income/expense, form fields, and whether each applies)
- `GET|POST /api/properties`, `GET|PUT|DELETE /api/properties/:id`
- `GET /api/overview` → portfolio rollup: `{ rows, totals }` — every property with its year summary (income/expense/net) and rent owed, plus per-kind totals (income, expense, net, outstanding, percent gain)
- `GET /api/properties/:id/summary` → income/expense totals
- `GET /api/properties/:id/breakdown` → totals by category
- `GET /api/properties/:id/outstanding` → rent owed by the current tenant
- `GET|POST /api/properties/:id/tenants`, `PUT|DELETE /api/tenants/:id`
- `POST /api/tenants/:id/leases`, `PUT|DELETE /api/leases/:id`
- `GET|POST /api/properties/:id/transactions`, `PUT|DELETE /api/transactions/:id`
- `GET /api/insurance`, `GET|POST /api/properties/:id/insurance`, `PUT|DELETE /api/insurance/:id`
- `POST /api/attachments` (multipart `file`), `GET /api/attachments/:id`

## Frontend (Next.js, port 3000)

```bash
cd frontend
npm install   # first time only
npm run dev
```

The frontend proxies `/api/*` requests to the backend at `http://localhost:8080`
via a rewrite in [next.config.ts](frontend/next.config.ts), so run both servers
together during development.

## Running both

In two terminals (or use the helper scripts):

```bash
# terminal 1
./scripts/backend.sh     # or: cd backend && cargo run

# terminal 2
./scripts/frontend.sh    # or: cd frontend && npm run dev
```

Then open http://localhost:3000.
