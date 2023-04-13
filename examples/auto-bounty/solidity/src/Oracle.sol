// SPDX-License-Identifier: CC-BY-1.0
pragma solidity ^0.8.17;

import "openzeppelin-contracts/access/AccessControl.sol";
import "openzeppelin-contracts/token/ERC20/IERC20.sol";
import "./IOracle.sol";

contract GHOracle is AccessControl, IGHOracle {
    bytes32 public constant OWNER_ROLE = keccak256("OWNER_ROLE");
    bytes32 public constant BACKEND_ROLE = keccak256("BACKEND_ROLE");

    IERC20 private feeToken;
    uint256 public queryFee;
    uint256 queryId;

    mapping(uint256 => bool) private responses;

    event NewFee(uint256 fee);

    constructor(address backend_wallet, IERC20 _feeToken, uint256 _queryFee) {
        feeToken = _feeToken;
        queryFee = _queryFee;
        queryId = 1;
        _grantRole(OWNER_ROLE, msg.sender);
        _grantRole(BACKEND_ROLE, backend_wallet);
    }

    function getFeeToken() public virtual override returns (IERC20) {
        return feeToken;
    }

    // Asks the backend the question
    // "was the PR at prURL merged and did it include commits from a user with ghUsername?"
    function query(string memory prUrl, string memory ghUsername) public virtual override returns (uint256) {
        if (queryFee > 0) {
            feeToken.transferFrom(msg.sender, address(this), queryFee);
        }
        uint256 id = queryId;
        queryId += 1;

        emit Query(id, prUrl, ghUsername);
        return id;
    }

    // Checks the response of query ID from the oracle.
    function checkResponse(uint256 id) public virtual override returns (bool) {
        return responses[id];
    }

    // Function the backend uses to respond to a query
    function respond(uint256 id, bool response) public onlyRole(BACKEND_ROLE) {
        responses[id] = response;

        emit Response(id, response);
    }

    // Claims the fees accumulated from queries
    function claimFees() public onlyRole(OWNER_ROLE) {
        uint256 amount = feeToken.balanceOf(address(this));
        feeToken.transfer(msg.sender, amount);
    }

    // Change the fee
    function updateFee(uint256 newFee) public onlyRole(OWNER_ROLE) {
        queryFee = newFee;

        emit NewFee(newFee);
    }

    // Other useful functions (not implemented for now due to hackathon timeline)
    // transfer ownership, rotate backend wallet, cleanup old responses
}
