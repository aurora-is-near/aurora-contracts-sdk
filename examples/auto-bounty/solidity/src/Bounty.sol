// SPDX-License-Identifier: CC-BY-1.0
pragma solidity ^0.8.17;

import "openzeppelin-contracts/token/ERC20/IERC20.sol";
import "./IOracle.sol";

contract BountyProgram {
    struct Bounty {
        // Sentinal field to check that this value is not the default one
        uint8 nonEmpty;
        string prUrl;
        IERC20 rewardAsset;
        uint256 rewardAmount;
        // Number of blocks for which this bounty can be locked before it must be claimed or
        // returned to being open.
        uint256 lockPeriod;
        // Address that created this bounty. This is stored so that it can be revoked by the owner.
        address owner;
    }

    struct LockedBounty {
        Bounty bounty;
        address payee;
        string ghUsername;
        uint256 lockHeight;
    }

    event Create(uint256 indexed id, IERC20 indexed rewardAsset, uint256 indexed rewardAmount, string prUrl);
    event Lock(uint256 indexed id, address payee, string ghUsername);
    event Claim(uint256 indexed id, string prUrl);
    event Revoke(uint256 indexed id, string prUrl);

    IGHOracle oracle;
    mapping(uint256 => Bounty) openBounties;
    mapping(uint256 => LockedBounty) lockedBounties;
    mapping(uint256 => uint256) queryIds;

    uint256 private bountyIdCounter;

    address constant ZERO = 0x0000000000000000000000000000000000000000;

    constructor(IGHOracle _oracle) {
        oracle = _oracle;
        bountyIdCounter = 1;

        // Need to approve the oracle to spend its fee token so that we can use it.
        IERC20 oracleFeeToken = _oracle.getFeeToken();
        oracleFeeToken.approve(address(_oracle), 0xffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff);
    }

    // Create a new open bounty. Called by maintainers of bounty programs.
    // This function causes the reward to be transferred to this contract.
    function createBounty(string calldata prUrl, IERC20 rewardAsset, uint256 rewardAmount, uint256 lockPeriod)
        public
        returns (uint256)
    {
        uint256 id = bountyIdCounter;
        bountyIdCounter += 1;

        rewardAsset.transferFrom(msg.sender, address(this), rewardAmount);
        openBounties[id] = Bounty(1, prUrl, rewardAsset, rewardAmount, lockPeriod, msg.sender);
        emit Create(id, rewardAsset, rewardAmount, prUrl);
        return id;
    }

    // Submits the intent to complete an open bounty.
    // Called by a developer looking to participate in the bounty program.
    function submitIntent(uint256 id, address payee, string calldata ghUsername) public {
        Bounty memory bounty = openBounties[id];

        if (bounty.nonEmpty == 0) {
            revert("Bounty ID not open.");
        }

        uint256 lockHeight = block.number;
        lockedBounties[id] = LockedBounty(bounty, payee, ghUsername, lockHeight);
        openBounties[id] = Bounty(0, "", IERC20(ZERO), 0, 0, ZERO);

        emit Lock(id, payee, ghUsername);
    }

    // Trigger the process to claim a locked bounty. This function returns `true`
    // if the process was started successfully, and `false` otherwise. The latter case
    // happens when the bounty was locked for longer than the duration set by the owner.
    // This function is called by a anyone (the developer has incentive to do it,
    // but there is no permission restriction) after the PR corresponding to the bounty is merged.
    function tryClaimBounty(uint256 id) public returns (bool) {
        LockedBounty memory bounty = lockedBounties[id];

        if (bounty.bounty.nonEmpty == 0) {
            revert("Bounty ID not locked.");
        }

        if (bounty.lockHeight + bounty.bounty.lockPeriod < block.number) {
            // Lock period elapsed, must be returned to open
            openBounties[id] = bounty.bounty;
            lockedBounties[id] = LockedBounty(Bounty(0, "", IERC20(ZERO), 0, 0, ZERO), ZERO, "", 0);
            return false;
        }

        uint256 queryId = oracle.query(bounty.bounty.prUrl, bounty.ghUsername);
        queryIds[id] = queryId;

        return true;
    }

    // Claim the bounty after the GitHub oracle has posted its response.
    // This function can be called by anyone, but again the developer has incentive to do it.
    function finishClaimBounty(uint256 bountyId) public returns (bool) {
        uint256 queryId = queryIds[bountyId];

        if (queryId == 0) {
            revert("No query for bounty ID");
        }

        bool response = oracle.checkResponse(queryId);

        // Reset mapping value since we have used the response now.
        queryIds[bountyId] = 0;

        if (response) {
            // If the oracle returns `true` then we pay out the bounty
            LockedBounty memory bounty = lockedBounties[bountyId];
            bounty.bounty.rewardAsset.transfer(bounty.payee, bounty.bounty.rewardAmount);
            lockedBounties[bountyId] = LockedBounty(Bounty(0, "", IERC20(ZERO), 0, 0, ZERO), ZERO, "", 0);
            emit Claim(bountyId, bounty.bounty.prUrl);
        }

        return response;
    }

    // Closes an open bounty without claiming it.
    function revokeBounty(uint256 id) public {
        Bounty memory bounty = openBounties[id];

        if (bounty.nonEmpty == 0) {
            revert("Bounty ID not open.");
        }

        if (msg.sender != bounty.owner) {
            revert("Only owner can revoke.");
        }

        // Return the reward to the owner.
        bounty.rewardAsset.transfer(bounty.owner, bounty.rewardAmount);

        openBounties[id] = Bounty(0, "", IERC20(ZERO), 0, 0, ZERO);
        emit Revoke(id, bounty.prUrl);
    }
}
