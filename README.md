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

> ℹ️ Note
>
> Each component has its own README file with more information.

| Crate  | Description    |
| ------ | -------------- |
| [`hose-blueprint`](./hose-blueprint) | A proc-macro for generating data types from CIP-57 schemas |
| [`hose-primitives`](./hose-primitives) | A library providing primitives and protocol parameters of Cardano |
| [`hose-submission`](./hose-submission) | A library for submitting and evaluating transactions to the Cardano blockchain |
| [`hose-txbuilder`](./hose-txbuilder) | A library for building and signing transactions in a composable way |

## Getting Started

> ⚠️ TODO
>
> There is a template project available at [`Liqwid-Labs/hose-template`](https://github.com/Liqwid-Labs/hose-template). 

## General design

Hose provides a semi-opinionated design, with a blessed path to follow, but it is also designed to be flexible and composable.

The general pattern of usage is as follows:

1. You define your on-chain scripts using Aiken.
2. You define your types using the `hose-blueprint` crate, loading the plutus.json file that the Aiken compiler generates.
3. You define your off-chain logic using the `hose-txbuilder` crate.
4. You define an API for your off-chain logic, and expose it to your frontend. Including submission using the `hose-submission` crate.
