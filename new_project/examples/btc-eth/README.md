# Hello-Swap

Hello-Swap is an example project that shows how to build applications on top of the C( )MIT protocol using TypeScript.

Hello-Swap uses the [comit-js-sdk](https://github.com/comit-network/comit-js-sdk) to communicate with the API of the C( )MIT reference implementation [comit-rs](https://github.com/comit-network/comit-rs).

## How to use it

### Set up the environment

Make sure that you have Docker installed before proceeding.

1. Get the latest version of [create-comit-app](https://github.com/comit-network/create-comit-app) by
    - downloading and unzipping the [latest release of create-comit-app](https://github.com/comit-network/create-comit-app/releases); or
    - cloning the repository and checking out the latest tagged release.
2. Run `create-comit-app start-env` inside the root directory of this project.
3. Wait until the environment is ready.

### Do an atomic swap

Make sure that you have yarn (or npm) installed before proceeding.

1. In a separate terminal, run `yarn install` (or `npm install`) to install dependencies.
2. Start the swap: `yarn start` (or `npm start`).
3. Hit `CTRL-C` once the swap is done.
