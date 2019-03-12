# treesnake
A tree search-based battlesnake.

## Building
1. Prerequisites: docker and docker-compose

1. Build the docker containers and run a game locally:
    ```
    git submodule update --init --recursive
    docker compose build
    docker compose up
    ```

1. View the locally running game. Look for link with the game ID printed by the rules service. For example:
    ```
    http://127.0.0.1:3000/?engine=http%3A%2F%2Flocalhost%3A4000&game=62befffb-711d-4ea1-99e6-3b6c17624d59
    ```
