// SPDX-License-Identifier: CC-BY-1.0
pragma solidity ^0.8.17;

import "openzeppelin-contracts/token/ERC20/IERC20.sol";

interface IGHOracle {
    event Query(uint256 indexed id, string prUrl, string ghUsername);
    event Response(uint256 indexed id, bool response);

    function query(string memory prUrl, string memory ghUsername) external returns (uint256);

    function checkResponse(uint256 id) external returns (bool);

    function getFeeToken() external returns (IERC20);
}
