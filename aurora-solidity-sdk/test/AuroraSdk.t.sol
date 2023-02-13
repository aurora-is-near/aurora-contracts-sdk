// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import "forge-std/Test.sol";
import "../src/AuroraSdk.sol";

contract AuroraSdkTest is Test {
    function testImplicitAuroraAddress() public {
        assertEq(
            AuroraSdk.implicitAuroraAddress("nearCrossContractCall"),
            address(0x516Cded1D16af10CAd47D6D49128E2eB7d27b372)
        );
    }
}
