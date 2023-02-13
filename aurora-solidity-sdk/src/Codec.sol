// SPDX-License-Identifier: CC-BY-1.0
pragma solidity ^0.8.17;

import "./Borsh.sol";
import "./Types.sol";
import "./Utils.sol";

/// Provide borsh serialization and deserialization for multiple types.
library Codec {
    using Borsh for Borsh.Data;

    function encodeU8(uint8 v) internal pure returns (bytes1) {
        return bytes1(v);
    }

    function encodeU16(uint16 v) internal pure returns (bytes2) {
        return bytes2(Utils.swapBytes2(v));
    }

    function encodeU32(uint32 v) public pure returns (bytes4) {
        return bytes4(Utils.swapBytes4(v));
    }

    function encodeU64(uint64 v) public pure returns (bytes8) {
        return bytes8(Utils.swapBytes8(v));
    }

    function encodeU128(uint128 v) public pure returns (bytes16) {
        return bytes16(Utils.swapBytes16(v));
    }

    /// Encode bytes into borsh. Use this method to encode strings as well.
    function encode(bytes memory value) public pure returns (bytes memory) {
        return abi.encodePacked(encodeU32(uint32(value.length)), bytes(value));
    }

    /// Encode Execution mode enum into borsh.
    function encode(ExecutionMode mode) public pure returns (bytes1) {
        return bytes1(uint8(mode));
    }

    /// Encode PromiseArgsVariant enum into borsh.
    function encode(PromiseArgsVariant mode) public pure returns (bytes1) {
        return bytes1(uint8(mode));
    }

    /// Encode base promise into borsh.
    function encode(PromiseCreateArgs memory nearPromise) public pure returns (bytes memory) {
        return abi.encodePacked(
            encode(bytes(nearPromise.targetAccountId)),
            encode(bytes(nearPromise.method)),
            encode(nearPromise.args),
            encodeU128(nearPromise.nearBalance),
            encodeU64(nearPromise.nearGas)
        );
    }

    /// Encode promise with callback into borsh.
    function encode(PromiseWithCallback memory nearPromise) public pure returns (bytes memory) {
        return abi.encodePacked(encode(nearPromise.base), encode(nearPromise.callback));
    }

    /// Encode create promise using borsh. The encoded data
    /// uses the same format that the Cross Contract Call precompile expects.
    function encodeCrossContractCallArgs(PromiseCreateArgs memory nearPromise, ExecutionMode mode)
        public
        pure
        returns (bytes memory)
    {
        return abi.encodePacked(encode(mode), encode(PromiseArgsVariant.Create), encode(nearPromise));
    }

    /// Encode promise with callback using borsh. The encoded data
    /// uses the same format that the Cross Contract Call precompile expects.
    function encodeCrossContractCallArgs(PromiseWithCallback memory nearPromise, ExecutionMode mode)
        public
        pure
        returns (bytes memory)
    {
        return abi.encodePacked(encode(mode), encode(PromiseArgsVariant.Callback), encode(nearPromise));
    }

    /// Decode promise result using borsh.
    function decodePromiseResult(Borsh.Data memory data) public pure returns (PromiseResult memory result) {
        result.status = PromiseResultStatus(data.decodeU8());
        if (result.status == PromiseResultStatus.Successful) {
            result.output = data.decodeBytes();
        }
    }

    /// Skip promise result from the buffer.
    function skipPromiseResult(Borsh.Data memory data) public pure {
        PromiseResultStatus status = PromiseResultStatus(uint8(data.decodeU8()));
        if (status == PromiseResultStatus.Successful) {
            data.skipBytes();
        }
    }
}
