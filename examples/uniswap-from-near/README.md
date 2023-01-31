# Calling Uniswap from Near

This is an example of a Near contract interacting with an EVM contract deployed on Aurora.
In this case the EVM contract is [Uniswap V3](https://docs.uniswap.org/contracts/v3/overview).
The full Uniswap API is complex, so this example only implements one function to illustrate the pattern.
The function implemented in this Near contract is [exactOutputSingle](https://docs.uniswap.org/contracts/v3/reference/periphery/SwapRouter#exactoutputsingle).
The `exactOutputSingle` function is used to perform a token swap using a single liquidity pool where the swap is constrained to give an exact amount of the "output" token within some price limitation of the "input" token.
The return value of `exactOutputSingle` is the amount of input token spent to make the swap.

The Near contract works as a proxy with a method called `exact_output_single` that takes the same input as Uniswap's `exactOutputSingle`.
The `exact_output_single` function returns a `Promise` because under the hood it is making a Near cross-contract call to the Aurora EVM where the Uniswap code is deployed.
This contract attaches a callback to the promise so that it can interpret the output obtained from Aurora and present it in a more Near-friendly way (i.e. encoding it JSON instead of a binary format).

This contract must own the assets it is using to swap inside the EVM.
Therefore a prerequisite to using this contract is to transfer assets to its EVM "implicit address".
See the main documentation in `docs/AuroraFromNear.md` for more information about this concept.

## Notes for Developers

### Workspace organization

This example is organized into two crates: the contract that would be deployed to a Near network, and a crate for integration testing of the contract.
This layout was chosen as opposed to making the integration tests part of a [tests directory](https://doc.rust-lang.org/book/ch11-03-test-organization.html#the-tests-directory) in the contract crate because the purpose of the integration tests is to test the compiled Wasm (i.e. binary) artifact of the contract as opposed to testing it as a Rust library.
My understanding of the [note in the Rust book about the tests directory](https://doc.rust-lang.org/book/ch11-03-test-organization.html#integration-tests-for-binary-crates) is that it meant for testing library integrations as opposed to binary integrations.
Thus I chose to factor the integration tests out as an entirely separate crate.
You may or may not make a different choice in your own projects, and that's ok.
