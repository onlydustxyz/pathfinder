HEROKU_APP_NAME="pathfinder-alpha-goerli"

docker buildx build --platform linux/amd64 -t $HEROKU_APP_NAME .
docker tag signingv2 registry.heroku.com/$HEROKU_APP_NAME/web
docker push registry.heroku.com/$HEROKU_APP_NAME/web

heroku container:release web -a $HEROKU_APP_NAME
