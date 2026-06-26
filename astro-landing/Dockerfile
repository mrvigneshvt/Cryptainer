# syntax=docker/dockerfile:1
# ---------- build the static site ----------
FROM node:22-alpine AS build
WORKDIR /app
COPY package.json package-lock.json ./
RUN npm ci
COPY . .
RUN npm run build

# ---------- serve it with nginx ----------
FROM nginx:1.27-alpine AS runtime
RUN rm -f /etc/nginx/conf.d/default.conf
COPY nginx.conf /etc/nginx/conf.d/cryptainer.conf
COPY --from=build /app/dist /usr/share/nginx/html
EXPOSE 80
# No HEALTHCHECK: under Docker Swarm (Dokploy) a failing container healthcheck
# gets the task killed and rescheduled. nginx serving static files needs none —
# the restart policy covers an actual crash. Re-add a real check later if wanted.
