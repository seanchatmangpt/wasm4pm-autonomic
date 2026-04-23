# syntax=docker/dockerfile:1.7
# Multi-stage build for matrix-tv, targeting Next.js `output: 'standalone'`.
# Expects: `npm run build` to have produced .next/standalone and .next/static.

# --- Stage 1: install + build ---
FROM node:20-alpine AS builder
WORKDIR /app

# Install dependencies with a reproducible lockfile.
COPY package.json package-lock.json* ./
RUN npm ci

# Copy source and build.
COPY . .
RUN npm run build

# --- Stage 2: runtime ---
FROM node:20-alpine AS runner
WORKDIR /app

ENV NODE_ENV=production
ENV NEXT_TELEMETRY_DISABLED=1
ENV PORT=3000
ENV HOSTNAME=0.0.0.0

# Create an unprivileged user.
RUN addgroup --system --gid 1001 nodejs \
 && adduser  --system --uid 1001 nextjs

# Copy the standalone bundle (includes only the node_modules Next.js needs).
COPY --from=builder --chown=nextjs:nodejs /app/.next/standalone ./
COPY --from=builder --chown=nextjs:nodejs /app/.next/static ./.next/static
COPY --from=builder --chown=nextjs:nodejs /app/public ./public

USER nextjs

EXPOSE 3000

HEALTHCHECK --interval=30s --timeout=5s --start-period=10s \
  CMD wget -q -O - "http://127.0.0.1:${PORT}/" >/dev/null || exit 1

CMD ["node", "server.js"]
