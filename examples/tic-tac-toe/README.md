# Tic-Tac-Toe XCC Example

This is a fully cross-chain example.
By which I mean that some of the smart contracts which comprise this dApp are in Rust deployed on Near and others are written in Solidity deployed on Aurora.
The two components communicate with each other using Aurora's XCC feature.

In this example the dApp is a tic-tac-toe game where the board state and game management are handled in Solidity, while the Computer opponent logic is in Rust.
The purpose of this example is to illustrate how you can use the strengths of both Aurora and Near to build one unified dApp, in the same way that a single Web 2.0 application can consist of both JavaScrip and WebAssembly components.
In this particular case the whole thing could have been written for either platform.
But you can imagine how a real use-case might involve solutions to multiple problems, some of which are easier to solve in Solidity (for example maybe there is a convenient OpenZepillin library) and others easier in Rust or in the Near ecosystem in general (for example maybe you want to take advantage of the protocol-level account abstraction).

