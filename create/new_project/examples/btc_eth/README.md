# Example for swapping Bitcoin for Ether

An example project that shows how to swap Bitcoin for Ether with COMIT.

In this example the taker and maker are split up in separate executable files.
Upon executing the maker and taker apps the execution will bounce between the maker and taker, making the example closer to a production app.

The example includes the simple negotiation protocol provided by the COMIT SDK.

It sets off at the point where the maker has already published an order for the taker.
It then consists of 5 steps:
1. The taker takes the order and starts the swap execution.
2. The taker funds the Ethereum HTLC.
3. The maker funds the Bitcoin HTLC.
4. The taker redeems the Bitcoin HTLC.
5. The maker redeems the Ethereum HTLC.

## How to run this example

Ensure that the environment is up in a separate terminal.

1. Run `yarn install` to install the necessary dependencies,
2. Run `yarn run maker` to start the maker app,
3. Run `yarn run taker` to start the taker app and initiate a swap,
4. Press `Enter` when asked to continue,
5. Profit!
