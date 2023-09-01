// SPDX-License-Identifier: CC-BY-1.0
pragma solidity ^0.8.17;

// See `docs/NearFromAurora.md` for more information on what is going on in this example.

import "openzeppelin-contracts/access/AccessControl.sol";
import "openzeppelin-contracts/token/ERC20/IERC20.sol";
import "openzeppelin-contracts/utils/Strings.sol";
import "./IEvmErc20.sol";
import {AuroraSdk, Codec, NEAR, PromiseCreateArgs, PromiseResult, PromiseResultStatus, PromiseWithCallback} from "aurora-sdk/AuroraSdk.sol";

uint64 constant FT_TRANSFER_CALL_NEAR_GAS = 35_000_000_000_000;

uint64 constant CALLBACK_NEAR_GAS = 130_000_000_000_000;

uint64 constant REFUND_NEAR_GAS = 35_000_000_000_000;

uint64 constant APPROVE_NEAR_GAS = 20_000_000_000_000;

contract FtRefund is AccessControl {
    using AuroraSdk for NEAR;
    using AuroraSdk for PromiseCreateArgs;
    using AuroraSdk for PromiseWithCallback;
    using Codec for bytes;

    bytes32 public constant CALLBACK_ROLE = keccak256("CALLBACK_ROLE");
    bytes16 private constant _SYMBOLS = "0123456789abcdef";

    IERC20 public wNEAR;
    string public nearAccountId;
    NEAR public near;

    constructor(string memory _nearAccountId, IERC20 _wNEAR) {
        nearAccountId = _nearAccountId;
        near = AuroraSdk.initNear(_wNEAR);
        wNEAR = _wNEAR;
        _grantRole(
            CALLBACK_ROLE,
            AuroraSdk.nearRepresentitiveImplicitAddress(address(this))
        );
    }

    // The purpose of this function is to get around a quirk of the XCC design.
    // The `CALLBACK_ROLE` is logically the same as this contract because it is
    // the implicit address of this contract's XCC account (and therefore all
    // transactions from `CALLBACK_ROLE` actually originated from this contract).
    // So this function is essentially just approving itself to spend its own tokens.
    // The reason its needed is because the addresses are different on the level of
    // the EVM even though they are logically the same entity as I have described above.
    // To summarize, this function is needed because
    // `nearRepresentitiveImplicitAddress(address(this)) != address(this)`, but we still
    // want to spend wNEAR in Aurora callbacks.
    function approveWNEAR() public {
        uint256 amount = 0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF;
        PromiseCreateArgs memory approveCall = near.auroraCall(
            address(this.wNEAR()),
            abi.encodeWithSelector(
                0x095ea7b3, // approve method selector
                address(this),
                amount
            ),
            0,
            APPROVE_NEAR_GAS
        );
        approveCall.transact();
    }

    // Given an ERC-20 token that was bridged to Aurora from Near,
    // this function will call the `ft_transfer_call` function on the
    // Near-native NEP-141 token that ERC-20 was bridged from.
    // This would be an important piece of an XCC-based integration into
    // a Near app such as ref.finance (allowing Aurora users to use the
    // Near app without creating a Near account themselves).
    // For the purpose of this example, the call includes a message to
    // force a refund back to the user because the point of this example
    // is to show how to refund the Aurora user in the case there is a failure
    // on the Near side.
    function ftTransferCall(
        IEvmErc20 token,
        string memory tokenId,
        uint128 amount
    ) public {
        token.transferFrom(msg.sender, address(this), amount);
        token.withdrawToNear(
            abi.encodePacked(AuroraSdk.nearRepresentative(address(this))),
            uint(amount)
        );

        bytes memory data = abi.encodePacked(
            "{",
            '"receiver_id": "',
            nearAccountId,
            '",',
            '"amount": "',
            Strings.toString(amount),
            '",',
            '"msg": "refund"',
            "}"
        );
        PromiseCreateArgs memory callFtTransfer = near.call(
            tokenId,
            "ft_transfer_call",
            data,
            1,
            FT_TRANSFER_CALL_NEAR_GAS
        );
        PromiseCreateArgs memory callback = near.auroraCall(
            address(this),
            abi.encodeWithSelector(
                this.ftTransferCallCallback.selector,
                msg.sender,
                tokenId,
                amount
            ),
            0,
            CALLBACK_NEAR_GAS
        );

        callFtTransfer.then(callback).transact();
    }

    // This function is the callback from `ftTransferCall` which refunds the user
    // in the case that `ft_transfer_call` did not comsume all the tokens the user
    // sent. This is accomplished by bridging the tokens returned to the XCC account
    // on Near back to Aurora via another `ft_transfer_call`.
    function ftTransferCallCallback(
        address sender,
        string memory tokenIdOnNear,
        uint128 amount
    ) public onlyRole(CALLBACK_ROLE) {
        PromiseResult memory promiseResult = AuroraSdk.promiseResult(0);
        uint128 refundAmount = 0;

        if (promiseResult.status != PromiseResultStatus.Successful) {
            // if Promise failed we need to do whole refund
            refundAmount = amount;
        } else {
            // else `ft_resolve_transfer` will return used amount of FT,
            // which we need to extract from original amount
            uint128 usedAmount = _stringToUint(string(promiseResult.output));
            refundAmount = amount - usedAmount;
        }

        if (refundAmount > 0) {
            bytes memory data = abi.encodePacked(
                "{",
                '"receiver_id": "',
                AuroraSdk.currentAccountId(),
                '",',
                '"amount": "',
                Strings.toString(refundAmount),
                '",',
                '"msg": "',
                _toHexString(uint160(sender), 20),
                '"}'
            );
            PromiseCreateArgs memory callFtTransfer = near.call(
                tokenIdOnNear,
                "ft_transfer_call",
                data,
                1,
                REFUND_NEAR_GAS
            );
            callFtTransfer.transact();
        }
    }

    function _toHexString(
        uint value,
        uint length
    ) internal pure returns (string memory) {
        bytes memory buffer = new bytes(2 * length);
        for (uint i = 2 * length; i > 0; i--) {
            buffer[i - 1] = _SYMBOLS[value & 0xf];
            value >>= 4;
        }
        require(value == 0, "Strings: hex length insufficient");
        return string(buffer);
    }

    function _stringToUint(
        string memory s
    ) internal pure returns (uint128 result) {
        bytes memory b = bytes(s);
        uint128 i;
        result = 0;
        for (i = 0; i < b.length; i++) {
            uint128 c = uint128(uint8(b[i]));
            if (c >= 48 && c <= 57) {
                result = result * 10 + (c - 48);
            }
        }
    }
}
