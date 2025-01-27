<div align="center">
    <picture>
        <source media="(prefers-color-scheme: dark)" srcset="./assets/hose-dark.svg">
        <source media="(prefers-color-scheme: light)" srcset="./assets/hose-light.svg">
        <img src="./assets/hose-light.svg" alt="hose" width="500">
    </picture>
    <hr />
        <h3 align="center" style="border-bottom: none">A modern off-chain framework</h3>
    <hr/>
</div>

## What is Hose?

Hose is a modern off-chain framework for building scalable and composable applications on top of the Cardano blockchain using Rust.

It is designed to work nicely with Aiken, a language for writing smart contracts on Cardano.

## Components

| Crate  | Description    |
| ------ | -------------- |
| [`hose-blueprint`](https://github.com/liqwid-labs/hose/tree/main/hose-blueprint) | A proc-macro for generating data types from CIP-57 schemas |
| [`hose-primitives`](https://github.com/liqwid-labs/hose/tree/main/hose-primitives) | A library providing primitives and protocol parameters of Cardano |
| [`hose-submission`](https://github.com/liqwid-labs/hose/tree/main/hose-submission) | A library for submitting and evaluating transactions to the Cardano blockchain |
| [`hose-txbuilder`](https://github.com/liqwid-labs/hose/tree/main/hose-txbuilder) | A library for building and signing transactions in a composable way |

