IMAGE=syncabull

docker:
	docker build -t $(IMAGE):api -f Dockerfile.api.dev .
	docker build -t $(IMAGE):web -f Dockerfile.client.dev .

docker-prod:
	docker build -t $(IMAGE):api -f Dockerfile.api .
	docker build -t $(IMAGE):web -f Dockerfile.client .
