# Fungible token refund

The purpose of this example is to demonstrate a proper way to do token bridging between Near and Aurora and to properly refund tokens that might have been stuck in the XCC contract otherwise.

More specifically, we have a Solidity contract `S` on Aurora which sends a fungible token (FT) on Near via calling `ft_transfer_call` on its contract `T`.
There is another receiving Near contract `R`, which denies the sent FT in its `ft_on_transfer` function, resulting in a refund from `T` to the XCC contract address.
The XCC contract now needs to bridge back the FT to the signing EVM wallet by calling `ft_transfer_call` on the token again, this time sending the tokens to Aurora.

A key aspect of this example is the fact that a callback to Aurora makes a further cross-contract call (XCC) which also spends NEAR (since `ft_transfer_call` requires 1 yoctoNEAR attached).
This is a little tricky to get right because the sender of the callback transaction to Aurora is derived from the XCC representative account on NEAR by hashing the account ID.
It is not equal to the address of the contract which caused the callback to be sent.
Therefore, additional setup is required to make this work; there must be an extra XCC call into Aurora where the derived account gives allowance to the Solidity contract to spend its WNEAR.
