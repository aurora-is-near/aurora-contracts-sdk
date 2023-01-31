# Aurora Contracts SDK

The purpose of this repository is to provide libraries that make it easy for developers to connect the [NEAR](https://near.org/) and [Aurora](https://aurora.dev/) ecosystems.
This includes a Rust library for contract developers on NEAR to call the Aurora EVM from their contract as well as a Solidity library for EVM contract developers to call out to the broader NEAR ecosystem from Aurora.

## Installation (contributors)

These steps are needed to contribute to the libraries in this repository.

The Solidity library is developed using the `forge` package within [Foundry](https://book.getfoundry.sh/index.html).
Forge uses [submodules](https://git-scm.com/book/en/v2/Git-Tools-Submodules) for contract dependencies.
Use `git submodule update --init --recursive` to ensure you clone the dependencies along with the code in this repo.

```shell
git clone https://github.com/aurora-is-near/aurora-contracts-sdk.git
cd aurora-contracts-sdk/
git submodule update --init --recursive
```

If you are contributing to the Solidity library, follow the [instructions to install Foundry](https://book.getfoundry.sh/getting-started/installation).
Then you should be able to run the tests

```shell
cd aurora-solidity-sdk/
forge test
```

### Navigating the examples with an IDE

The examples in this repository are separate from the main Aurora SDK library and we do not include them in a top-level Cargo workspace.
This has the unfortunate side-effect that the examples are not parsed when opening an IDE like VS Code in the repository root.
If you are playing with the example in an IDE we recommend opening the directory for that example directly (for example `examples/uniswap-from-near`).
