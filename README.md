# odds-api

The repository contains a partial integration of [OddsPapi - B2B Sports Betting Odds API](https://docs.oddspapi.io/).

Documentation about the design can be found [in this design doc](./design.md) but as a design doc it does not contain all the implementation nuances.

## Usage

The program is intended to be run in docker however it requires an API key `ODDS_API_KEY` for the `client` service. Check the [docker compose file](./docker/docker-compose.yml) for more info.

You may use the docker commands directly from the [justfile](./justfile) if you do not want to install `just`.

### Build

Build the program

```sh
just
```

### Start Services

The following command starts the client and pulls in minimal OTeL for logging and a grafana UI to inspect the logs upon first use.

When the services start navigate to `http://localhost:3000/dashboards` and click on `Logs` to see the system operate.

It starts by pulling in a snapshot from OddsPapi therefore it may take 30 seconds before updates begin streaming from their websocket.

```sh
just run
```

### Stop Services

To stop the services without deleting generated logs

```sh
just stop
```

### Purge Services

To stop and delete generated logs (but not delete the docker images)

```sh
just clean
```
