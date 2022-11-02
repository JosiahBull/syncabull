IMAGE=syncabull

docker:
	docker build -t $(IMAGE):api -f Dockerfile.api.dev .
	docker build -t $(IMAGE):client -f Dockerfile.client.dev .
	docker build -t $(IAMGE):web -f ./app/Dockerfile.dev ./app

docker-prod:
	docker build -t $(IMAGE):api -f Dockerfile.api .
	docker build -t $(IMAGE):client -f Dockerfile.client .
	docker build -t $(IAMGE):web -f ./app/Dockerfile ./app
