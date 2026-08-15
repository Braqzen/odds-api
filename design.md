# OddsPapi integration design

## Goal

Build a reliable in-memory state of tradable fixtures and odds from OddsPapi. OddsPapi is the required source of truth, but feed mistakes such as wrong prices, incorrect fixture mappings, stale data, and out-of-order updates must not automatically become tradable state.

## Architecture

A single service runs concurrent tasks for REST snapshots, WebSocket updates, risk decisions, and in-memory state. Tasks communicate through channels so feed ingestion, validation, and state mutation remain separate.

1. Load fixtures and odds from REST to create the initial state.
2. Receive fixture, bookmaker, and odds updates over WebSocket.
3. Pass snapshot data and live updates through the same risk engine.
4. Store only accepted data in memory.
5. When the feed requests a new snapshot, stop applying WebSocket updates and build the replacement state separately from REST.
6. Replace the active state atomically only after the complete snapshot has passed validation, then resume WebSocket processing.

## Risk decisions

Every item receives one decision:

- `Enable`: add or update accepted state.
- `Disable`: remove a valid but non-tradable item and any state that depends on it.
- `Ignore`: drop malformed, inconsistent, or suspicious data without overwriting accepted state.

Updates are checked individually, so one bad quote does not block valid quotes in the same message.

## Safeguards

- Accept only configured sports and fixtures with pregame or live status.
- Require the fixture and bookmaker to exist before accepting a quote.
- Require `oddsId` to equal `{fixtureId}:{bookmaker}:{outcomeId}:{playerId}`.
- Disable bookmakers with no odds, stale odds, suspension, or participant rotation.
- Disable inactive quotes and inactive or unknown markets.
- Ignore updates older than the stored `changedAt`.
- Ignore zero and non-finite prices.
- Treat prices outside configured limits or movements beyond configured thresholds as non-tradable.
- Remove all dependent quotes when a fixture or bookmaker becomes unsafe.
- Ignore WebSocket updates while rebuilding state.
- Exit on WebSocket failure instead of continuing with stale data.

Price limits and movement thresholds are not currently configured, so they remain an open safeguard rather than an enforced check. Even with those limits, a fully self-consistent but factually wrong fixture or plausible price cannot be detected from OddsPapi alone. That requires an independent data source or a separate execution-level price check.

## Further safety extensions

The following controls would provide additional protection before this state is used for trading. They are not currently implemented:

- Remove accepted data from memory when it exceeds a configured maximum age.
- Configure acceptable price ranges and maximum price movement thresholds.
- Periodically revalidate the fixture, market, bookmaker, price, and freshness of stored data.
- Compare mappings and prices with an independent source where the potential loss justifies it.

## Trade boundary

This service currently builds validated state and does not submit orders but it could be extended to expose the state or use it to create orders.

## Failure simulation

Feed ingestion should be replaceable with recorded or generated payloads so snapshots and WebSocket sequences can be tested without live endpoints. Tests should cover malformed, stale, reordered, duplicate, and partial data, along with rebuilds and connection failures This repository does not currently provide that simulation layer.
