// SPDX-License-Identifier: CC-BY-1.0
pragma solidity ^0.8.17;

import "openzeppelin-contracts/token/ERC20/IERC20.sol";
import "./IOracle.sol";

contract BountyProgram {
    struct Bounty {
        uint256 id;
        string prUrl;
        IERC20 rewardAsset;
        uint256 rewardAmount;
        // Number of blocks for which this bounty can be locked before it must be claimed or
        // returned to being open.
        uint256 lockPeriod;
        // Address that created this bounty. This is stored so that it can be revoked by the owner.
        address owner;
    }

    struct Application {
        uint256 bountId;
        address payee;
        string ghUsername;
        string comment;
    }

    struct LockedBounty {
        Bounty bounty;
        address payee;
        string ghUsername;
        uint256 lockHeight;
    }

    // Doubly linked list bookkeeping to enable iteration over open bounties
    uint256 private numOpenBounties;
    uint256 private lastOpenBounty;
    mapping(uint256 => uint256) private nextOpenBounty;
    mapping(uint256 => uint256) private previousOpenBounty;

    // Doubly linked list bookkeeping to enable iteration over locked bounties
    uint256 private numLockedBounties;
    uint256 private lastLockedBounty;
    mapping(uint256 => uint256) private nextLockedBounty;
    mapping(uint256 => uint256) private previousLockedBounty;

    // Linked list to bookkeeping to enable iteration over applications
    mapping(uint256 => uint256) private numApplications;
    mapping(uint256 => address) private lastApplication;
    mapping(uint256 => mapping(address => address)) private previousApplication;

    event Create(
        uint256 indexed id, address indexed owner, IERC20 indexed rewardAsset, uint256 rewardAmount, string prUrl
    );
    event Apply(uint256 indexed id, address payee, string ghUsername);
    event Approve(uint256 indexed id, address payee, string ghUsername);
    event Claim(uint256 indexed id, string prUrl);
    event Revoke(uint256 indexed id, string prUrl);

    IGHOracle oracle;
    mapping(uint256 => Bounty) public openBounties;
    mapping(uint256 => mapping(address => Application)) private applications;
    mapping(uint256 => LockedBounty) public lockedBounties;
    mapping(uint256 => uint256) private queryIds;

    uint256 private bountyIdCounter;

    address constant ZERO = 0x0000000000000000000000000000000000000000;

    constructor(IGHOracle _oracle) {
        oracle = _oracle;
        bountyIdCounter = 1;
        numOpenBounties = 0;
        numLockedBounties = 0;

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
        address owner = msg.sender;
        openBounties[id] = Bounty(id, prUrl, rewardAsset, rewardAmount, lockPeriod, owner);
        pushOpenBounty(id);
        emit Create(id, owner, rewardAsset, rewardAmount, prUrl);
        return id;
    }

    // Submits the intent to complete an open bounty.
    // Called by a developer looking to participate in the bounty program.
    function submitApplication(uint256 id, address payee, string calldata ghUsername, string calldata comment) public {
        Bounty memory bounty = openBounties[id];

        if (bounty.id == 0) {
            revert("Bounty ID not open.");
        }

        applications[id][payee] = Application(id, payee, ghUsername, comment);
        numApplications[id] += 1;
        address prevApplication = lastApplication[id];
        lastApplication[id] = payee;
        previousApplication[id][payee] = prevApplication;

        emit Apply(id, payee, ghUsername);
    }

    // Approves an application to work on a bounty.
    // Only the owner of the bounty (the creator) can call this function
    function approveApplication(uint256 id, address payee) public {
        Bounty memory bounty = openBounties[id];

        if (bounty.id == 0) {
            revert("Bounty ID not open.");
        }
        if (bounty.owner != msg.sender) {
            revert("Owner must approve.");
        }

        Application memory application = applications[id][payee];

        if (application.bountId == 0) {
            revert("Application not found.");
        }

        uint256 lockHeight = block.number;
        lockedBounties[id] = LockedBounty(bounty, payee, application.ghUsername, lockHeight);
        openBounties[id] = Bounty(0, "", IERC20(ZERO), 0, 0, ZERO);
        removeOpenBounty(id);
        pushLockedBounty(id);

        emit Approve(id, payee, application.ghUsername);
    }

    // Trigger the process to claim a locked bounty. This function returns `true`
    // if the process was started successfully, and `false` otherwise. The latter case
    // happens when the bounty was locked for longer than the duration set by the owner.
    // This function is called by a anyone (the developer has incentive to do it,
    // but there is no permission restriction) after the PR corresponding to the bounty is merged.
    function tryClaimBounty(uint256 id) public returns (bool) {
        LockedBounty memory bounty = lockedBounties[id];

        if (bounty.bounty.id == 0) {
            revert("Bounty ID not locked.");
        }

        if (bounty.lockHeight + bounty.bounty.lockPeriod < block.number) {
            // Lock period elapsed, must be returned to open
            openBounties[id] = bounty.bounty;
            pushOpenBounty(id);
            lockedBounties[id] = LockedBounty(Bounty(0, "", IERC20(ZERO), 0, 0, ZERO), ZERO, "", 0);
            removeLockedBounty(id);
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
            removeLockedBounty(bountyId);
            emit Claim(bountyId, bounty.bounty.prUrl);
        }

        return response;
    }

    // Closes an open bounty without claiming it.
    function revokeBounty(uint256 id) public {
        Bounty memory bounty = openBounties[id];

        if (bounty.id == 0) {
            revert("Bounty ID not open.");
        }

        if (msg.sender != bounty.owner) {
            revert("Only owner can revoke.");
        }

        // Return the reward to the owner.
        bounty.rewardAsset.transfer(bounty.owner, bounty.rewardAmount);

        openBounties[id] = Bounty(0, "", IERC20(ZERO), 0, 0, ZERO);
        removeOpenBounty(id);
        emit Revoke(id, bounty.prUrl);
    }

    // List applications for bounty with ID
    function listApplications(uint256 id) public view returns (Application[] memory) {
        uint256 n = numApplications[id];
        Application[] memory result = new Application[](n);

        address payee = lastApplication[id];
        for (uint256 i = 0; i < n; i++) {
            result[i] = applications[id][payee];
            payee = previousApplication[id][payee];
        }

        return result;
    }

    function listOpenBounties() public view returns (Bounty[] memory) {
        Bounty[] memory result = new Bounty[](numOpenBounties);
        uint256 id = lastOpenBounty;

        for (uint256 i = 0; i < numOpenBounties; i++) {
            result[i] = openBounties[id];
            id = previousOpenBounty[id];
        }

        return result;
    }

    function listLockedBounties() public view returns (LockedBounty[] memory) {
        LockedBounty[] memory result = new LockedBounty[](numLockedBounties);
        uint256 id = lastLockedBounty;

        for (uint256 i = 0; i < numLockedBounties; i++) {
            result[i] = lockedBounties[id];
            id = previousLockedBounty[id];
        }

        return result;
    }

    // List all the bounties created by a specifc address. The return type is `LockedBounty`, but
    // not all the bounties may be locked. Those that are still open to applications will have a
    // `lockedHeight` of 0.
    function listBountiesCreatedBy(address owner) public view returns (LockedBounty[] memory) {
        uint256[] memory ids = new uint256[](numOpenBounties + numLockedBounties);
        uint256 n = 0;
        uint256 lastLocked = 0;

        uint256 id = lastLockedBounty;
        for (uint256 i = 0; i < numLockedBounties; i++) {
            LockedBounty memory bounty = lockedBounties[id];
            if (bounty.bounty.owner == owner) {
                ids[n] = id;
                lastLocked += 1;
                n += 1;
            }
            id = previousLockedBounty[id];
        }

        id = lastOpenBounty;
        for (uint256 i = 0; i < numOpenBounties; i++) {
            Bounty memory bounty = openBounties[id];
            if (bounty.owner == owner) {
                ids[n] = id;
                n += 1;
            }
            id = previousOpenBounty[id];
        }

        LockedBounty[] memory result = new LockedBounty[](n);
        for (uint256 i = 0; i < lastLocked; i++) {
            id = ids[i];
            result[i] = lockedBounties[id];
        }
        for (uint256 i = lastLocked; i < n; i++) {
            id = ids[i];
            result[i] = LockedBounty(openBounties[id], ZERO, "", 0);
        }

        return result;
    }

    // List all open applications created for a specific address.
    function listApplicationsBy(address payee) public view returns (Application[] memory) {
        uint256[] memory ids = new uint256[](numOpenBounties);
        uint256 n = 0;
        uint256 id = lastOpenBounty;

        for (uint256 i = 0; i < numOpenBounties; i++) {
            Application memory application = applications[id][payee];
            if (application.bountId > 0) {
                ids[n] = id;
                n += 1;
            }
            id = previousOpenBounty[id];
        }

        Application[] memory result = new Application[](n);
        for (uint256 i = 0; i < n; i++) {
            id = ids[i];
            result[i] = applications[id][payee];
        }
        return result;
    }

    // List all bounties a given address is currently approved to work on.
    function listActiveBountiesBy(address payee) public view returns (LockedBounty[] memory) {
        uint256[] memory ids = new uint256[](numLockedBounties);
        uint256 n = 0;
        uint256 id = lastLockedBounty;

        for (uint256 i = 0; i < numLockedBounties; i++) {
            LockedBounty memory bounty = lockedBounties[id];
            if (bounty.payee == payee) {
                ids[n] = id;
                n += 1;
            }
            id = previousLockedBounty[id];
        }

        LockedBounty[] memory result = new LockedBounty[](n);
        for (uint256 i = 0; i < n; i++) {
            id = ids[i];
            result[i] = lockedBounties[id];
        }
        return result;
    }

    // Add a new open bounty to the end of the list
    function pushOpenBounty(uint256 id) private {
        numOpenBounties += 1;
        nextOpenBounty[lastOpenBounty] = id;
        previousOpenBounty[id] = lastOpenBounty;
        lastOpenBounty = id;
    }

    // Remove open bounty from the list
    function removeOpenBounty(uint256 id) private {
        numOpenBounties -= 1;
        if (id == lastOpenBounty) {
            // Special case, pop off the end of the list
            lastOpenBounty = previousOpenBounty[id];
            nextOpenBounty[lastOpenBounty] = 0;
            return;
        }

        // Assume list looks like `prev <=> id <=> next`.
        // After removing `id` it looks like `prev <=> next
        uint256 prev = previousOpenBounty[id];
        uint256 next = nextOpenBounty[id];
        nextOpenBounty[prev] = next;
        previousOpenBounty[next] = prev;
    }

    // Add a new open bounty to the end of the list
    function pushLockedBounty(uint256 id) private {
        numLockedBounties += 1;
        nextLockedBounty[lastLockedBounty] = id;
        previousLockedBounty[id] = lastLockedBounty;
        lastLockedBounty = id;
    }

    // Remove open bounty from the list
    function removeLockedBounty(uint256 id) private {
        numLockedBounties -= 1;
        if (id == lastLockedBounty) {
            // Special case, pop off the end of the list
            lastLockedBounty = previousLockedBounty[id];
            nextLockedBounty[lastLockedBounty] = 0;
            return;
        }

        // Assume list looks like `prev <=> id <=> next`.
        // After removing `id` it looks like `prev <=> next
        uint256 prev = previousLockedBounty[id];
        uint256 next = nextLockedBounty[id];
        nextLockedBounty[prev] = next;
        previousLockedBounty[next] = prev;
    }
}
