<a href="https://comit.network/docs/getting-started/create-comit-app">
<img src="logo.svg" height="120px">
</a>

---

[COMIT](https://comit.network) is an open protocol facilitating cross-blockchain applications.
With [COMIT](https://comit.network) you can for examples exchange Bitcoin for any Erc20 token directly with another person.

This repository contains everything needed to do an atomic swap (locally) on your machine.

If you wish to do an atomic swap on your machine or to integrate COMIT into an application (e.g. a DEX) please take a look at the [Getting Started section](https://comit.network/docs/getting-started/create-comit-app/) of the COMIT documentation.
If you have any questions, feel free to [reach out to the team in our Gitter chat](https://gitter.im/comit-network/community)!

[![License](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](https://opensource.org/licenses/Apache-2.0)
[![Gitter chat](https://badges.gitter.im/gitterHQ/gitter.png)](https://gitter.im/comit-network/community)

# Create Comit App

## Getting Started

`create-comit-app` enables you to setup a local development environment for running and developing Javascript apps on top of COMIT.
`create-comit-app` comes with demos and examples to demonstrate how to use the [comit-js-sdk](https://github.com/comit-network/comit-js-sdk).

If you already have [Docker](https://www.docker.com/), [npm](https://www.npmjs.com/) and [yarn](https://yarnpkg.com/) installed you are ready to run a demo-swap on your machine:

[![asciicast](https://asciinema.org/a/298948.png)](https://asciinema.org/a/298948)

For more details please check the [Detailed Guidelines](#detailed-guidelines) and the [Getting Started section](https://comit.network/docs/getting-started/create-comit-app/) of the COMIT documentation.
If you have any question feel free to [reach out to us on Gitter](https://gitter.im/comit-network/community)!

---

## Detailed Guidelines

### 1 - Install Docker

#### Unix

Install docker through your package manager.
Make sure the unix-socket of the docker daemon is accessible.

#### Windows

Windows users have the choice between Docker Toolbox (the "old" docker) and Docker for Windows (the "new" docker).
Docker for Windows requires Windows 10 Pro, whereas Docker Toolbox also works on older versions of Windows and Windows 10 Home.

##### Docker Toolbox

Install Docker Toolbox and start the virtual machine.
Double check that the environment variables `DOCKER_HOST`, `DOCKER_CERT_PATH` and `DOCKER_TLS_VERIFY` have been set.

##### Docker for Windows

To use create-comit-app with Docker for Windows please follow these steps:

1. Set the `DOCKER_HOST` variable to the docker daemon endpoint. You can find that in the docker control panel, should be something like `tcp://127.0.0.1:2375`.
2. Disable the TLS verification of the docker daemon endpoint: In your docker control panel: Settings > General > Expose daemon on tcp... without TLS

### 2 - Install yarn & nodeJS

Install them either from the [website](https://classic.yarnpkg.com/en/docs/install/) or through your package manager.

### 3 - Create your first project!

`create-comit-app` contains [demos](https://github.com/comit-network/create-comit-app/tree/master/create/new_project/demos) and [examples](https://github.com/comit-network/create-comit-app/tree/master/create/new_project/examples). 
Demos demonstrate how a swap works in a very simple manner, whereas examples go more into details and add maker, taker and negotiation.

Here is how you set up a project and run [demos](https://github.com/comit-network/create-comit-app/tree/master/create/new_project/demos) and [examples](https://github.com/comit-network/create-comit-app/tree/master/create/new_project/examples):

1. `yarn create comit-app <your-app-name>`,
2. `cd <your-app-name>`, `yarn install` and `yarn start-env` to start blockchain and COMIT nodes,
3. Run the [btc-eth demo](https://github.com/comit-network/create-comit-app/tree/master/create/new_project/demos/btc_eth)
    1. Navigate to the separate-apps demo directory `cd ./demos/btc_eth`,
    2. `yarn install` to install dependencies,
    3. `yarn swap` to execute the swap,
4. Run the [btc-eth example](https://github.com/comit-network/create-comit-app/tree/master/create/new_project/examples/btc_eth) (in separate terminals for the taker and maker):
    1. Navigate to the separate-apps example app directory `cd ./examples/btc_eth`,
    2. `yarn install` to install dependencies,
    3. `yarn run maker` to run the maker app,
    4. `yarn run taker` to run the taker app,
    5. Follow the steps be hitting `Return` to complete the swap.

---

# Appendix

Important: You don't have to follow this section, the above section is actually sufficient.

## Appendix A: Build the project yourself

`create-comit-app` is a rust project separated in two modules:

1. **create**: Contains the logic for setting up a new `comit-app` project through yarn.
2. **scripts**: Contains the logic of starting a local dev-environment.
    1. Starts up parity Ethereum node
    2. Start up bitcoind Bitcoin node
    3. Starts up two [cnd](https://github.com/comit-network/comit-rs) nodes.

To build the project yourself:

1. Install Docker,
2. Install Rust: `curl https://sh.rustup.rs -sSf | sh`,
3. Checkout the repo: `git clone https://github.com/comit-network/create-comit-app/; cd create-comit-app`,
4. Build and install: `make install`.
