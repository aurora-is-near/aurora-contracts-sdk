// SPDX-License-Identifier: CC-BY-1.0
pragma solidity ^0.8.17;

// See `docs/NearFromAurora.md` for more information on what is going on in this example.

import "openzeppelin-contracts/access/AccessControl.sol";
import "openzeppelin-contracts/token/ERC20/IERC20.sol";
import {
    AuroraSdk,
    Codec,
    NEAR,
    PromiseCreateArgs,
    PromiseResultStatus,
    PromiseWithCallback
} from "aurora-sdk/AuroraSdk.sol";

// When making a call to another NEAR contract, you must specify how much NEAR gas
// will be attached to the call (this is simlar to the `gas` argument in the EVM `call` opcode).
// The typical unit of has on Near is the teragas (Tgas), where 1 Tgas = 10^12 gas.
// For example, the block gas limit on NEAR is 1000 Tgas, and the transaction gas limit is 300 Tgas.
uint64 constant SET_NEAR_GAS = 50_000_000_000_000;
uint64 constant SET_CALLBACK_NEAR_GAS = 10_000_000_000_000;

// We use the Open Zeppelin access control feature because the methods of this contract should
// not be open to arbitrary addresses.
contract SocialDB is AccessControl {
    using AuroraSdk for NEAR;
    using AuroraSdk for PromiseCreateArgs;
    using AuroraSdk for PromiseWithCallback;
    using Codec for bytes;

    bytes32 public constant SETTER_ROLE = keccak256("SETTER_ROLE");
    bytes32 public constant CALLBACK_ROLE = keccak256("CALLBACK_ROLE");

    IERC20 public wNEAR;
    string public socialdbAccountId;
    NEAR public near;

    constructor(string memory _socialdbAccountId, IERC20 _wNEAR) {
        socialdbAccountId = _socialdbAccountId;
        near = AuroraSdk.initNear(_wNEAR);
        wNEAR = _wNEAR;
        _grantRole(SETTER_ROLE, msg.sender);
        _grantRole(CALLBACK_ROLE, AuroraSdk.nearRepresentitiveImplicitAddress(address(this)));
    }

    // Exposes the [set interface](https://github.com/NearSocial/social-db/tree/39016e654739b0a3e8cb7ffaea4b03157c4aea6e#storing-data)
    // of the SocialDB contract. This function is access controlled because it is important that
    // only authorized users can instruct keys to be set in the DB.
    // An amount of wNEAR is also required for this call to cover the storage cost of the data
    // being persisted on Near.
    function set(uint128 attachedNear, bytes memory data) public onlyRole(SETTER_ROLE) {
        wNEAR.transferFrom(msg.sender, address(this), attachedNear);

        PromiseCreateArgs memory callSet =
            near.call(socialdbAccountId, "set", data, attachedNear, SET_NEAR_GAS);
        PromiseCreateArgs memory callback =
            near.auroraCall(address(this), abi.encodePacked(this.setCallback.selector), 0, SET_CALLBACK_NEAR_GAS);

        callSet.then(callback).transact();
    }

    // This function is not meant to be called by an externally owned account (EOA) on Aurora.
    // It should only be invoked as a callback from the main `set` method above. This is
    // the reason why this function has separate access control from `set`.
    function setCallback() public onlyRole(CALLBACK_ROLE) {
        if (AuroraSdk.promiseResult(0).status != PromiseResultStatus.Successful) {
            revert("Call to set failed");
        }
    }
}
