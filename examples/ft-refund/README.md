# Fungible token refund

The purpose of this example is to demonstrate a proper way to do token bridging between Near and Aurora and to properly refund tokens that might have been stuck in the XCC contract otherwise.

More specifically, we have a Solidity contract `S` on Aurora which sends a fungible token (FT) on Near via calling `ft_transfer_call` on its contract `T`. There is another receiving Near Rust contract `R`, which denies the sent FT in its `ft_on_transfer` function, resulting in a refund from `T` to the XCC contract address. The XCC contract now needs to bridge back the FT to the signing EVM wallet.
