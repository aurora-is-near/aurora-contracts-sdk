# Bounty program with GitHub Oracle

The idea with this project is to use on-chain automation to make admistering a bounty program easier.

There are two contracts here: one is the contract which stores information about the available bounties, and the other is an oracle for GitHub.

The intended workflow is as follows.

Suppose the organization proposing the bounty has a GitHub repository with the appropriate branch protections
(for example PRs to the `main` branch require CI checks to pass and multiple approvals from maintainers).
This means PRs can only be merged when they really fulfill the requirements.
The person giving the bounty creates a draft PR where the work that is out for bounty is outlined (in the PR decription, but also perhaps has some `TODO` comments in the relevant parts of the code).
They then submit the URL for this PR to the bounty contract along with the parameters of the bounty (how much it is worth, the timeline during which it needs to be completed etc).
When the bounty is submitted, the bounty contract also takes the assets that will be paid out if the work is completed.

A developer can look at the bounties available on this contract and decide they want to complete one.
They submit their intent to complete the bounty to the smart contract.
This transaction includes the address that will receive the assets on completion and the GitHub username they will use for the work
(specifying the a username could be important if the repository requires commits to be signed (verified on GitHub) because then we know that person did the work).
Once a transaction for intent has been sent that bounty will be "locked" for a period of time, giving the developer a chance to complete the work without competition for the reward.
If they fail to complete the work before the time elapses then they will need to re-submit their intent, or else another developer can submit their own intent instead.
If the developer completes the work and the PR is merged (recall this requires approval by the organization's own engineers) then the developer claims the bounty by submitting another transaction to the smart contract.
To confirm the work was completed, the bounty contract uses the GitHub oracle to check that the PR was merged and contained commits from the provided username.
If this check passes then the bounty contract releases the funds to the address the developer specified when they submitted their intent.

The reason to allow the address that receives the bounty to be different from `msg.sender` is in the case that a group of developers want to work on the bounty and
they do not trust one of them to fairly distribute the funds if they receive them personally. Since any address can be specified, they can have the funds paid out to another smart contract
which has pre-agreed upon logic for distributing the reward among the participants. I.e. this allows composing the bounty workflow with other workflows.
