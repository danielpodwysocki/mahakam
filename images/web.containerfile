FROM node:24-alpine AS build

WORKDIR /app
COPY frontends/web/package*.json ./
RUN npm ci

COPY frontends/web/ ./
RUN npm run build

FROM node:24-alpine AS prod

WORKDIR /app
COPY --from=build /app/.output ./

EXPOSE 3001
ENV PORT=3001
CMD ["node", "server/index.mjs"]
