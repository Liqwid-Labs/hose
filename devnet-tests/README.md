# Devnet Tests

This crate contains the tests that test hose on the devnet.

## Running the tests

To run the tests, you need to set the following environment variables:

- `PRIVATE_KEY_HEX`: The private key to use for the wallet. This should be a 64 character hex string.
- `NETWORK`: The network to use. Either `testnet` or `mainnet`. For devnet tests, you should set this to `testnet`.
- `DB_PATH`: The path to the database to use for the tests. You can use `./db` for the database path.
- `NODE_HOST`: The host and port of the node to use for the tests.
- `OGMIOS_URL`: The URL of the ogmios server to use for the tests. For devnet tests, you likely should set this to `http://localhost:1337`.
- `GENESIS_BYRON_PATH`: The path to the byron genesis file. This file can be found in the local-testnet repository.
- `GENESIS_SHELLEY_PATH`: The path to the shelley genesis file. This file can be found in the local-testnet repository.

Then, you can run the tests with:

```sh
cargo test -p devnet-tests --bin devnet-tests
```