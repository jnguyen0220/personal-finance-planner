# syntax=docker/dockerfile:1

# ---- Dependencies ----
FROM node:22-bookworm-slim AS deps
WORKDIR /app
COPY package.json package-lock.json* ./
RUN npm ci

# ---- Build ----
FROM node:22-bookworm-slim AS builder
WORKDIR /app
COPY --from=deps /app/node_modules ./node_modules
COPY . .
ENV NEXT_TELEMETRY_DISABLED=1
# Next.js bakes rewrite destinations into the build, so BACKEND_URL must be
# present at build time, not just at runtime.
ARG BACKEND_URL
ENV BACKEND_URL=${BACKEND_URL}
RUN npm run build

# ---- Runtime ----
FROM node:22-bookworm-slim AS runtime
WORKDIR /app
ENV NODE_ENV=production
ENV NEXT_TELEMETRY_DISABLED=1
ENV PORT=3000
ENV HOSTNAME=0.0.0.0

RUN useradd --system --create-home --uid 10001 app

# Next.js standalone output bundles only what is needed to run.
COPY --from=builder --chown=app:app /app/public ./public
COPY --from=builder --chown=app:app /app/.next/standalone ./
COPY --from=builder --chown=app:app /app/.next/static ./.next/static

USER app
EXPOSE 3000

CMD ["node", "server.js"]
