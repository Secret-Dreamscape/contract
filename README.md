# Secret Dreamscape

Welcome to Secret DreamScape, a mysterious journey into the world between sleeping and waking.

Secret DreamScape is a multiplayer card game that will test your ability to out-wit and out-think your opponents with the victor taking unimaginable spoils.

## Working With Us

We are a lean globally distributed team and are currently looking for passionate developers to partner with. We actually have several awesome projects in our pipeline. One of which has recently received grant funding through SCRT Labs to start building enhanced developer tools for the network. We will also be running a validator node very soon for the network. If you like our work and want to work regularily with us then we'd love to hear from you. We have stock options available for qualified development partners. You can find us in the weekly Secret Network Developer Committee calls on Discord. Drop in and introduce yourself! https://scrt.network/committees

## Game Rules

Each player is dealt a hand of letter cards and can place bets against their opponents. Once a bet is agreed on, community cards are played and each player can bet on the strength of the highest scoring word they can create. When all bets are agreed on, the showdown happens and to the winner go the riches. Gameplay ends when one player eliminates all of the others by reducing their hit points to zero.

# The deck
The deck coinsists of the classical scrabble.

## Credits

- Secret Contract and Web Frontend Development: Danny Morabito (@thelmuxkriovar)
- Game idea and playtesting: Gino Bernardi (@zorostang) and Daniel Allen (@danieldallen)
- Card design: Daniel Allen (@danieldallen)
- Leadership during development: Gino Bernardi (@zorostang)
- Demo video creation: Daniel Allen (@danieldallen)

## Demo

The game is live on mainnet [here](https://play.secretdreamscape.com/)!

A recording of the gameplay of the game can be found [here](https://youtu.be/qRRicifO8xI)

## Getting Started

Secret Dreamscape is built on the Secret Network, a Cosmos-based blockchain with a focus on privacy and security and the only live blockchain capable of protecting user privacy.

The codebase is split into 6 public git repositories and 1 private frontend repository. 

* contract
* stamper
* jackpot
* phonebook
* user-card-settings
* NFTs-first-edition

Setup instructions are provided below and also in the readme's of the mentinoned contracts

This brings us to compiling and storing the contract. You will need to have docker and rust setup and working on your system to get started. Provided that you have cloned the contract repo and that you've entered that folder in your terminal, you can follow the following steps to get started.

First run a local secret testnet

```sh
docker run -it --rm \
  -p 26657:26657 -p 26656:26656 -p 1317:1317 \
  -v $(pwd):/contract \
  --name secretdev enigmampc/secret-network-sw-dev:v1.2.2-1
```

This will setup a local testnet running version 1.2.2-1 of the secret network sw-dev, and will expose ports 26657, 26656 and 1317 to the outside world. Port 26656 is RPC and can be used to communicate with the testnet from outside the container (such as with secretcli), while 1317 is the LCD, a REST-based API that secret.js uses to send commands to a contract and get data from the contract.

The next step will be to compile the contract down to a wasm binary, we've setup a simple makefile that does just that, so all you need to do is run the following:

```sh
make clean build
```

and the makefile will take care of the rest.

The next step will be to tell the testnet to store your contract so that it can be accessed. In orcer to do that you would run the following commands:

```sh
docker exec -it secretdev /bin/bash
cd /contract
secretd tx compute store contract.wasm.gz --from a --gas 10000000 -y --keyring-backend test
```

which will get a shell into the newly created local testnet and store the contract form the contract.wasm.gz file into the testnet's blockchain.

The secret network doesn't provide a simple container to setup a faucet for your testnet, but you may want to look at their instructions [here](https://github.com/scrtlabs/testnet-faucet) if you want to venture into that realm, otherwise you can always send money to a new wallet by running

```sh
secretd tx bank send a "ADDRESS HERE" 100000000uscrt
```

## Technologies used in the Frontend

The frontend repo will remain private for now. However, we're looking to collaborate with some talented frontend developers. If you're interested in diving into the technologies described below, please reach out!

We make use of a number of different technologies in this project.

- mobx, a state management library that makes it easy to handle the state for the game, and make sure that everyhing is kept in place
- phaser, a game engine for the web designed to make it east to develop all kinds of games
- secretjs, a library that allows for the interactions with the secret network.
- svelte, for wrapping our common components into reusable web components
