# hose-blueprint

## Introduction

This crate provides a proc-macro that generates types for your on-chain datums and scripts.

## Usage

Add this to your `Cargo.toml`:

```toml
[dependencies]
hose-blueprint = { git = "https://github.com/Liqwid-Labs/hose" }
```

Then, add this to your `lib.rs`:

```rust
use hose_blueprint::blueprint;

blueprint!("./path/to/plutus.json");
```

This macro will generate types that match your on-chain!

## Details

If you are interested in how this works, take a look at the [CIP-57 proposal](https://cips.cardano.org/cip/CIP-57). We also expose a binary that can be used to look at the generated types in [./bin](./bin). You can try running it with: 

```bash
cargo run --bin hose-blueprint -- ./path/to/plutus.json
```

Note however that we need `bat` and `rustfmt` to be installed for this to work.
