// SPDX-License-Identifier: CC-BY-1.0
pragma solidity ^0.8.17;

library Utils {
    function swapBytes2(uint16 v) internal pure returns (uint16) {
        return (v << 8) | (v >> 8);
    }

    function swapBytes4(uint32 v) internal pure returns (uint32) {
        v = ((v & 0x00ff00ff) << 8) | ((v & 0xff00ff00) >> 8);
        return (v << 16) | (v >> 16);
    }

    function swapBytes8(uint64 v) internal pure returns (uint64) {
        v = ((v & 0x00ff00ff00ff00ff) << 8) | ((v & 0xff00ff00ff00ff00) >> 8);
        v = ((v & 0x0000ffff0000ffff) << 16) | ((v & 0xffff0000ffff0000) >> 16);
        return (v << 32) | (v >> 32);
    }

    function swapBytes16(uint128 v) internal pure returns (uint128) {
        v = ((v & 0x00ff00ff00ff00ff00ff00ff00ff00ff) << 8) | ((v & 0xff00ff00ff00ff00ff00ff00ff00ff00) >> 8);
        v = ((v & 0x0000ffff0000ffff0000ffff0000ffff) << 16) | ((v & 0xffff0000ffff0000ffff0000ffff0000) >> 16);
        v = ((v & 0x00000000ffffffff00000000ffffffff) << 32) | ((v & 0xffffffff00000000ffffffff00000000) >> 32);
        return (v << 64) | (v >> 64);
    }

    function swapBytes32(uint256 v) internal pure returns (uint256) {
        v = ((v & 0x00ff00ff00ff00ff00ff00ff00ff00ff00ff00ff00ff00ff00ff00ff00ff00ff) << 8)
            | ((v & 0xff00ff00ff00ff00ff00ff00ff00ff00ff00ff00ff00ff00ff00ff00ff00ff00) >> 8);
        v = ((v & 0x0000ffff0000ffff0000ffff0000ffff0000ffff0000ffff0000ffff0000ffff) << 16)
            | ((v & 0xffff0000ffff0000ffff0000ffff0000ffff0000ffff0000ffff0000ffff0000) >> 16);
        v = ((v & 0x00000000ffffffff00000000ffffffff00000000ffffffff00000000ffffffff) << 32)
            | ((v & 0xffffffff00000000ffffffff00000000ffffffff00000000ffffffff00000000) >> 32);
        v = ((v & 0x0000000000000000ffffffffffffffff0000000000000000ffffffffffffffff) << 64)
            | ((v & 0xffffffffffffffff0000000000000000ffffffffffffffff0000000000000000) >> 64);
        return (v << 128) | (v >> 128);
    }

    function readMemory(uint256 ptr) internal pure returns (uint256 res) {
        assembly {
            res := mload(ptr)
        }
    }

    function writeMemory(uint256 ptr, uint256 value) internal pure {
        assembly {
            mstore(ptr, value)
        }
    }

    function memoryToBytes(uint256 ptr, uint256 length) internal pure returns (bytes memory res) {
        if (length != 0) {
            assembly {
                // 0x40 is the address of free memory pointer.
                res := mload(0x40)
                let end :=
                    add(res, and(add(length, 63), 0xffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffe0))
                // end = res + 32 + 32 * ceil(length / 32).
                mstore(0x40, end)
                mstore(res, length)
                let destPtr := add(res, 32)
                // prettier-ignore
                for {} 1 {} {
                    mstore(destPtr, mload(ptr))
                    destPtr := add(destPtr, 32)
                    if eq(destPtr, end) { break }
                    ptr := add(ptr, 32)
                }
            }
        }
    }

    function keccak256Raw(uint256 ptr, uint256 length) internal pure returns (bytes32 res) {
        assembly {
            res := keccak256(ptr, length)
        }
    }

    function sha256Raw(uint256 ptr, uint256 length) internal view returns (bytes32 res) {
        assembly {
            // 2 is the address of SHA256 precompiled contract.
            // First 64 bytes of memory can be used as scratch space.
            let ret := staticcall(gas(), 2, ptr, length, 0, 32)
            // If the call to SHA256 precompile ran out of gas, burn any gas that remains.
            // prettier-ignore
            for {} iszero(ret) {} {}
            res := mload(0)
        }
    }

    /// Convert array of bytes to hexadecimal string.
    /// https://ethereum.stackexchange.com/a/126928/45323
    function bytesToHex(bytes memory buffer) public pure returns (string memory) {
        // Fixed buffer size for hexadecimal convertion
        bytes memory converted = new bytes(buffer.length * 2);

        bytes memory _base = "0123456789abcdef";

        for (uint256 i = 0; i < buffer.length; i++) {
            converted[i * 2] = _base[uint8(buffer[i]) / _base.length];
            converted[i * 2 + 1] = _base[uint8(buffer[i]) % _base.length];
        }

        return string(converted);
    }
}
