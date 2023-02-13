// SPDX-License-Identifier: CC-BY-1.0
pragma solidity ^0.8.17;

/// Basic NEAR promise.
struct PromiseCreateArgs {
    /// Account id of the target contract to be called.
    string targetAccountId;
    /// Method in the contract to be called
    string method;
    /// Payload to be passed to the method as input.
    bytes args;
    /// Amount of NEAR tokens to attach to the call. This will
    /// be charged from the caller in wNEAR.
    uint128 nearBalance;
    /// Amount of gas to attach to the call.
    uint64 nearGas;
}

enum PromiseArgsVariant
/// Basic NEAR promise
{
    Create,
    /// NEAR promise with a callback attached.
    Callback,
    /// Description of arbitrary NEAR promise. Allows applying combinators
    /// recursively, multiple action types and batched actions.
    /// See https://nomicon.io/RuntimeSpec/Components/BindingsSpec/PromisesAPI
    /// for a complete description of what is possible.
    Recursive
}

/// Combine two base promises using NEAR combinator `then`.
struct PromiseWithCallback {
    /// Initial promise to be triggered.
    PromiseCreateArgs base;
    /// Second promise that is executed after the execution of `base`.
    /// In particular this promise will have access to the result of
    /// the `base` promise.
    PromiseCreateArgs callback;
}

enum ExecutionMode
/// Eager mode means that the promise WILL be executed in a single
/// NEAR transaction.
{
    Eager,
    /// Lazy mode means that the promise WILL be scheduled for execution
    /// and a separate interaction is required to trigger this execution.
    Lazy
}

enum PromiseResultStatus
/// This status should not be reachable.
{
    NotReady,
    /// The promise was executed successfully.
    Successful,
    /// The promise execution failed.
    Failed
}

struct PromiseResult {
    /// Status result of the promise execution.
    PromiseResultStatus status;
    /// If the status is successful, output contains the output of the promise.
    /// Otherwise the output field MUST be ignored.
    bytes output;
}
