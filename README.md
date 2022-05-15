# Syncabull
A small website + api which allows clients to download data from their google photos.

<!-- TODO: readme -->

## Development Setup
```bash
cp .example.dev.env .env
nano .env # Update port and host if desired

docker volume create syncabull-dev-pgdata

docker-compose -f docker-compose.dev.yml --env-file .env up
```

## Simple Production Deployment
```bash
cp .example.env .env
nano .env # Update domain and email for letsencrypt config, set a long random database password

docker volume create syncabull-pgdata
docker-compose --env-file .env up
```