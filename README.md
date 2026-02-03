<div align="center">
    <picture>
        <source media="(prefers-color-scheme: dark)" srcset="./assets/hose-dark.svg">
        <source media="(prefers-color-scheme: light)" srcset="./assets/hose-light.svg">
        <img src="./assets/hose-light.svg" alt="hose" width="500">
    </picture>
    <hr />
        <h3 align="center" style="border-bottom: none">A modern transaction builder</h3>
    <hr/>
</div>

## What is Hose?

Hose is a modern transaction builder, combining with [`hydrant`](https://github.com/liqwid-labs/hydrant) (indexer) and [`polymer`](https://github.com/liqwid-labs/polymer) (datum macro), to provide a modern Cardano off-chain framework using Rust.

It is designed to work nicely with Aiken, a language for writing smart contracts on Cardano.

## See also

- [`polymer`](https://github.com/liqwid-labs/polymer) - A library for generating Rust types from a CIP-57 blueprint schema.
- [`hydrant`](https://github.com/liqwid-labs/hydrant) - Embeddable & extensible chain-indexer for Cardano 

## Getting Started

> ⚠️ TODO
>
> There is a template project available at [`Liqwid-Labs/hose-template`](https://github.com/Liqwid-Labs/hose-template). 

## General design

Hose provides a semi-opinionated design, with a blessed path to follow, but all components are designed to be independent and composable.

The general pattern of usage is as follows:

1. You define your on-chain scripts using Aiken.
2. You define your types using [`polymer`](https://github.com/liqwid-labs/polymer), loading the plutus.json file that the Aiken compiler generates.
3. You define your off-chain logic using hose.
4. You distill the chain via an embedded indexer using [`hydrant`](https://github.com/liqwid-labs/hydrant).
4. You define an API for your off-chain logic, and expose it to your frontend.
