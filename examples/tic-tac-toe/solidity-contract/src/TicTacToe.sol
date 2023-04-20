// SPDX-License-Identifier: CC-BY-1.0
pragma solidity ^0.8.17;

// TODO: high level description

import "openzeppelin-contracts/access/AccessControl.sol";
import "openzeppelin-contracts/token/ERC20/IERC20.sol";
import {
    AuroraSdk,
    Codec,
    NEAR,
    PromiseCreateArgs,
    PromiseResultStatus,
    PromiseWithCallback,
    PromiseResult
} from "aurora-sdk/AuroraSdk.sol";

// When making a call to another NEAR contract, you must specify how much NEAR gas
// will be attached to the call (this is simlar to the `gas` argument in the EVM `call` opcode).
// The typical unit of has on Near is the teragas (Tgas), where 1 Tgas = 10^12 gas.
// For example, the block gas limit on NEAR is 1000 Tgas, and the transaction gas limit is 300 Tgas.
uint64 constant GET_MOVE_NEAR_GAS = 5_000_000_000_000;
uint64 constant COMPUTER_TURN_CALLBACK_NEAR_GAS = 30_000_000_000_000;

// A deposit is required to create a wNEAR sub-account. See `docs/NearFromAurora.md` for details.
uint128 constant STORAGE_DEPOSIT = 2_000_000_000_000_000_000_000_000;

// We use the Open Zeppelin access control feature because the methods of this contract should
// not be open to arbitrary addresses.
contract TicTacToe is AccessControl {
    using AuroraSdk for NEAR;
    using AuroraSdk for PromiseCreateArgs;
    using AuroraSdk for PromiseWithCallback;
    using AuroraSdk for PromiseResult;
    using Codec for bytes;

    bytes32 public constant CALLBACK_ROLE = keccak256("CALLBACK_ROLE");
    bytes32 public constant OWNER_ROLE = keccak256("OWNER_ROLE");

    IERC20 public wNEAR;
    string public ticTacToeAccountId;
    NEAR public near;

    // Map holding the state of all on-going tic-tac-toe games.
    // The state is represented as a uint256 because it is the "word" size on the EVM.
    // The last 9 bytes are used to represent the game state. These 9 bytes represent the
    // row-major tic tac toe grid. `0x00` means empty, `0x01` means X and `0x11` means O.
    // Higher bytes are set if the game is over.
    mapping(address => uint256) games;

    // Map storing all legal moves. This is filled in the constructor.
    mapping(uint256 => uint256) legalMoves;

    event Turn(address indexed player, string state);

    constructor(string memory _ticTacToeAccountId, IERC20 _wNEAR) {
        ticTacToeAccountId = _ticTacToeAccountId;
        near = AuroraSdk.initNear(_wNEAR);
        wNEAR = _wNEAR;
        _grantRole(OWNER_ROLE, msg.sender);
        _grantRole(CALLBACK_ROLE, AuroraSdk.nearRepresentitiveImplicitAddress(address(this)));

        // X in the top left
        legalMoves[0x010000000000000000] = 1;
        // O in the top left
        legalMoves[0x110000000000000000] = 1;

        // X in the top center
        legalMoves[0x000100000000000000] = 1;
        // O in the top center
        legalMoves[0x001100000000000000] = 1;

        // X in the top right
        legalMoves[0x000001000000000000] = 1;
        // O in the top right
        legalMoves[0x000011000000000000] = 1;

        // X in the middle left
        legalMoves[0x000000010000000000] = 1;
        // O in the middle left
        legalMoves[0x000000110000000000] = 1;

        // X in the middle ceter
        legalMoves[0x000000000100000000] = 1;
        // O in the middle center
        legalMoves[0x000000001100000000] = 1;

        // X in the middle right
        legalMoves[0x000000000001000000] = 1;
        // O in the middle right
        legalMoves[0x000000000011000000] = 1;

        // X in the bottom left
        legalMoves[0x000000000000010000] = 1;
        // O in the bottom left
        legalMoves[0x000000000000110000] = 1;

        // X in the bottom center
        legalMoves[0x000000000000000100] = 1;
        // O in the bottom center
        legalMoves[0x000000000000001100] = 1;

        // X in the bottom right
        legalMoves[0x000000000000000001] = 1;
        // O in the bottom right
        legalMoves[0x000000000000000011] = 1;
    }

    // Fund and create the XCC sub-account for this contract.
    // This only needs to happen once because calls to the tic tac toe Near contract are
    // stateless (and therefore do not require addition wNEAR funds).
    function init() public onlyRole(OWNER_ROLE) {
        // Fund the account from owner.
        wNEAR.transferFrom(msg.sender, address(this), STORAGE_DEPOSIT);

        // Make a cross-contract call to trigger sub-account creation.
        bytes memory data = abi.encodePacked("{\"state\":\".........\"}");
        PromiseCreateArgs memory callGetMove = near.call(ticTacToeAccountId, "get_move", data, 0, GET_MOVE_NEAR_GAS);
        callGetMove.transact();
    }

    // Start a new game where `player_preference = 0` means player goes second (plays O) and
    // `player_preference > 0` means the plater goes first (plays X).
    function newGame(uint256 player_preference) public {
        address player = msg.sender;
        games[player] = 0;
        if (player_preference == 0) {
            takeComputerTurn(player, 0);
        }
    }

    function takePlayerTurn(uint256 move) public {
        address player = msg.sender;
        uint256 currentState = games[player];
        require(currentState < 0x1000000000000000000, "Game Over");
        require(legalMoves[move] > 0, "Invalid move");
        require(move & currentState == 0, "Move at filled cell");
        currentState ^= move;
        games[player] = currentState;
        takeComputerTurn(player, currentState);
    }

    function getGameState(address player) public view returns (uint256) {
        return games[player];
    }

    // Call the tic tac toe contract on NEAR to make a move.
    function takeComputerTurn(address player, uint256 initialState) private {
        bytes memory data = abi.encodePacked("{\"state\":\"", encodeStateForNear(initialState), "\"}");

        PromiseCreateArgs memory callGetMove = near.call(ticTacToeAccountId, "get_move", data, 0, GET_MOVE_NEAR_GAS);
        PromiseCreateArgs memory callback = near.auroraCall(
            address(this),
            abi.encodeWithSelector(this.computerTurnCallback.selector, player),
            0,
            COMPUTER_TURN_CALLBACK_NEAR_GAS
        );

        callGetMove.then(callback).transact();
    }

    // Get the result of calling the NEAR contract. Update the internal state of this contract.
    function computerTurnCallback(address player) public onlyRole(CALLBACK_ROLE) {
        PromiseResult memory result = AuroraSdk.promiseResult(0);

        if (result.status != PromiseResultStatus.Successful) {
            revert("Tic tac toe Near call failed");
        }

        // output is of the form `{"updated_state":"<NINE_STATE_BYTES>","winner":"CellState::<X|O|Empty>"}`
        // where the `winner` field is optional.
        uint256 updatedState = decodeNearState(result.output);

        if (result.output.length > 37) {
            // Indicate the game is over by setting some higher bytes
            updatedState ^= 0x1100000000000000000000;
        }

        games[player] = updatedState;

        emit Turn(player, string(result.output));
    }

    function encodeStateForNear(uint256 state) private pure returns (string memory) {
        bytes memory stateBytes = abi.encodePacked(state);
        bytes memory output = new bytes(9);
        for (uint256 i = 23; i < 32; i++) {
            if (stateBytes[i] == 0x11) {
                output[i - 23] = "O";
            } else if (stateBytes[i] == 0x01) {
                output[i - 23] = "X";
            } else {
                output[i - 23] = ".";
            }
        }
        return string(output);
    }

    function decodeNearState(bytes memory stateBytes) private pure returns (uint256) {
        bytes memory output = new bytes(32);
        for (uint256 i = 18; i < 27; i++) {
            if (stateBytes[i] == "O") {
                output[i + 5] = 0x11;
            } else if (stateBytes[i] == "X") {
                output[i + 5] = 0x01;
            } else {
                output[i + 5] = 0x00;
            }
        }
        return uint256(bytes32(output));
    }
}
