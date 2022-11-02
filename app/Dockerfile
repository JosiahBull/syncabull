FROM node:16-alpine3.16 as builder

WORKDIR /app

# Move package and install *first* in order to cache the dependency install step
COPY ./package.json ./yarn.lock ./

RUN yarn \
  --prefer-offline \
  --frozen-lockfile \
  --non-interactive

COPY . .

RUN yarn build

ENV HOST 0.0.0.0
ENV PORT 3000
ENV NODE_ENV production

EXPOSE 3000

CMD [ "yarn", "start" ]
