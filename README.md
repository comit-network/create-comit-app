<a href="https://comit.network/docs/getting-started/create-comit-app">
<img src="logo.svg" height="120px">
</a>

---

[COMIT](https://comit.network) is an open protocol facilitating cross-blockchain applications.
With [COMIT](https://comit.network) you can for examples exchange Bitcoin for any Erc20 token directly with another person.

This repository contains everything needed to do an atomic swap (locally) on your machine.

If you wish to do an atomic swap on your machine or to integrate COMIT into an application (e.g. a DEX) please take a look at the [Getting Started section](https://comit.network/docs/getting-started/create-comit-app/) of the COMIT documentation.
If you have any question please [reach out to the team in our Gitter chat](https://gitter.im/comit-network/community)!


[![License](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](https://opensource.org/licenses/Apache-2.0)
[![Gitter chat](https://badges.gitter.im/gitterHQ/gitter.png)](https://gitter.im/comit-network/community)

# Create Comit App

`create-comit-app` enables you to setup a local development environment for running and developing Javascript apps on top of COMIT.
`create-comit-app` comes with demos and examples to demonstrate how to use the [comit-js-sdk](https://github.com/comit-network/comit-js-sdk).

[![asciicast](https://asciinema.org/a/298948.png)](https://asciinema.org/a/298948)

If you have any question please [reach out to the team in our Gitter chat](https://gitter.im/comit-network/community)!

## 1 - Install docker

### Unix

Install docker through your package manager.
Make sure the unix-socket of the docker daemon is accessible.

### Windows

Windows users have the choice between Docker Toolbox (the "old" docker) and Docker for Windows (the "new" docker).
Docker for Windows requires Windows 10 Pro, whereas Docker Toolbox also works on older versions of Windows and Windows 10 Home.

#### Docker Toolbox

Install Docker Toolbox and start the virtual machine.
Double check that the environment variables `DOCKER_HOST`, `DOCKER_CERT_PATH` and `DOCKER_TLS_VERIFY` have been set.

#### Docker for Windows

To use create-comit-app with Docker for Windows please follow these steps:

1. Set the `DOCKER_HOST` variable to the docker daemon endpoint. You can find that in the docker control panel, should be something like `tcp://127.0.0.1:2375`.
2. Disable the TLS verification of the docker daemon endpoint: In your docker control panel: Settings > General > Expose daemon on tcp... without TLS

## 2 - Install yarn & nodeJS

Install them either from the website or through your package manager.

## 3 - Create your first project!

1. `yarn create comit-app <your-app-name>`,
2. `cd <your-app-name>`, `yarn install` and `yarn start-env` to start blockchain and COMIT nodes,
3. Run the [btc-eth](https://github.com/comit-network/create-comit-app/tree/master/new_project/examples/btc_eth) example (in separate terminals for the taker and maker):
    1. Navigate to the separate-apps example app directory `cd ./examples/btc_eth`,
    2. `yarn install` to install dependencies,
    3. `yarn run maker` to run the maker app,
    4. `yarn run taker` to run the taker app,
    5. Follow the steps be hitting `Return` to complete the swap.

You can find additional examples in the [examples](https://github.com/comit-network/create-comit-app/tree/master/new_project/examples) directory that is created as part of step 1.

# Appendix

Important: You don't have to follow this section, the above section is actually sufficient.

## Appendix A: Build the project yourself

1. Install Docker,
2. Install Rust: `curl https://sh.rustup.rs -sSf | sh`,
3. Checkout the repo: `git clone https://github.com/comit-network/create-comit-app/; cd create-comit-app`,
4. Build and install: `make install`.
