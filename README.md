# Syncabull

A small website + api which allows clients to download data from their google photos.

## Installation

```bash
# Server
cp .example.env .env
nano .env
docker-compose --env-file .env up -d && docker-compose logs -f

# Client
cd client
cp .example.env .env
export $(grep -v '^#' .env | xargs)
cargo run --release
```
