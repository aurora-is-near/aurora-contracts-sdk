// SPDX-License-Identifier: CC-BY-1.0
pragma solidity ^0.8.17;

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

uint64 constant SET_NEAR_GAS = 50_000_000_000_000;
uint64 constant SET_CALLBACK_NEAR_GAS = 10_000_000_000_000;

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

    function set(uint128 attachedNear, bytes memory data) public onlyRole(SETTER_ROLE) {
        wNEAR.transferFrom(msg.sender, address(this), attachedNear);

        PromiseCreateArgs memory callSet =
            near.call(socialdbAccountId, "set", data, attachedNear, SET_NEAR_GAS);
        PromiseCreateArgs memory callback =
            near.auroraCall(address(this), abi.encodePacked(this.setCallback.selector), 0, SET_CALLBACK_NEAR_GAS);

        callSet.then(callback).transact();
    }

    function setCallback() public onlyRole(CALLBACK_ROLE) {
        if (AuroraSdk.promiseResult(0).status != PromiseResultStatus.Successful) {
            revert("Call to set failed");
        }
    }
}
