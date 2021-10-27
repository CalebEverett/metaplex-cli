[![build status](https://github.com/CalebEverett/metaplex-cli/actions/workflows/build.yml/badge.svg)](https://github.com/CalebEverett/metaplex-cli/actions/workflows/build.yml)


# Metaplex Command Line Interface

This is a command line interface for creating and managing non-fungible tokens on the [Solana blockchain](https://solana.com/) through the [Metaplex programs](https://metaplex.com/), including uploading assets and metadata to the [Arweave](https://www.arweave.org/) permaweb. The current application can be used to create single tokens and upload single files, but will be enhanced in the coming days to support bulk operations and create and manage auctions and candy stores.

## Implemented commands

* `mint-create`: create a new token mint - same command as in spl-token, included here for convenience. 
* `mint-supply`: display supply of tokens from mint - same command as in spl-token, included here for convenience. 
* `mint-info`: display information for an existing mint account.
* `metadata-info`: display information for an existing metadata account.
* `metadata-create`: create a new metadata account for an existing mint, including creators and shares.
* `metadata-update`: update an existing metadata account by providing either a mint or metadata account address and providing values for one or more updatable fields:
    * new_update_authority
    * name
    * symbol
    * uri
    * seller_fee_basis_points
    * creators
    * primary_sale_happened
* `nft-create`: create a de novo nft including mint, token account, metadata account and master edition.
* `arweave`: upload files to the Arweave permaweb. Current commands allow uploading and fetching transactions.

## Getting Started

1. `git clone git@github.com:CalebEverett/metaplex-cli.git`
1. `cd metaplex-cli`
2. `cargo build`
3. `cargo run -- -help`

## Usage

### Create an NFT

```
cargo-run -- nft-create
```

This brief command kicks off the process of minting a token, creating a token account, minting one token to the account, creating a metadata account and finally, creating a master edition. You can add one or more values for name, symbol, uri, or creators after `nft-create`. Creator shares have to sum to 100, values are specified in whole integer percentages and if you provide any creators, one of them has to be the same as the update authority. The update authority defaults to the wallet address in the local Solana config, but you can provide another one by flag.

### View Metadata Info

```
cargo-run -- metadata-info Cbg5o1tarienqQeQ8FcS6inGw2edrZ73znyYhVFtXa8b

```

 You can pass either the metadata account or token mint address. Without any values provided for the metadata fields, this produces a blank NFT, but it can be updated later since the `--immutable` flag wasn't set.

 ```

Address: qHFGEW7vnW61Arupoo4WcV3uqoDiRRhFv3fzHGPusBt
Key: MetadataV1
Update Authority: 61mVTaw6hBtwWnSaGXRSJePFWEQqipeCka3evytEVNUp
Mint: Cbg5o1tarienqQeQ8FcS6inGw2edrZ73znyYhVFtXa8b
Primary Sale Happened: false
Is Mutable: true
Edition Nonce: 255
Name:
Symbol:
Uri:
Seller Fee Basis Points: 0
```

If you want the output in json, you can add `json` or `json-compact` to the `--output` flag.


```
cargo run -- metadata-info --output json Cbg5o1tarienqQeQ8FcS6inGw2edrZ73znyYhVFtXa8b

```

```
{
  "address": "qHFGEW7vnW61Arupoo4WcV3uqoDiRRhFv3fzHGPusBt",
  "metadata": {
    "key": "MetadataV1",
    "updateAuthority": "61mVTaw6hBtwWnSaGXRSJePFWEQqipeCka3evytEVNUp",
    "mint": "Cbg5o1tarienqQeQ8FcS6inGw2edrZ73znyYhVFtXa8b",
    "name": "",
    "symbol": "",
    "uri": "",
    "sellerFeeBasisPoints": "0",
    "creators": null,
    "primarySaleHappened": false,
    "isMutable": true,
    "editionNonce": "255"
  }
}
```
### Update Metadata

Metadata can be updated with the `metadata-update` command, providing at least one additional flag with the value to be updated. Creators are specified with an address followed by a colon and then the respective share. For example, if we wanted to update the above metadata, we could enter:

```
cargo run -- metadata-update qHFGEW7vnW61Arupoo4WcV3uqoDiRRhFv3fzHGPusBt \
    --name "My NFT" --symbol "NFT" \
    --creators 61mVTaw6hBtwWnSaGXRSJePFWEQqipeCka3evytEVNUp:50 7oHuVGKc5ZA2tdJX2xLxfUuZPf4RWMsEuNFWkByZNNs7:50 \
    --uri ipfs://tbd
```

Same as with `nft-create`, you can provide either the token mint address or the metadata account address.


```
cargo run -- metadata-info Cbg5o1tarienqQeQ8FcS6inGw2edrZ73znyYhVFtXa8b
```

to see the updates:

```
Address: qHFGEW7vnW61Arupoo4WcV3uqoDiRRhFv3fzHGPusBt
Key: MetadataV1
Update Authority: 61mVTaw6hBtwWnSaGXRSJePFWEQqipeCka3evytEVNUp
Mint: Cbg5o1tarienqQeQ8FcS6inGw2edrZ73znyYhVFtXa8b
Primary Sale Happened: false
Is Mutable: true
Edition Nonce: 255
Name: My NFT
Symbol: NFT
Uri: ipfs://tbd
Seller Fee Basis Points: 0
Creators: 2
  [0] Address: 61mVTaw6hBtwWnSaGXRSJePFWEQqipeCka3evytEVNUp
      Verified: false
      Share: 50

  [1] Address: 7oHuVGKc5ZA2tdJX2xLxfUuZPf4RWMsEuNFWkByZNNs7
      Verified: false
      Share: 50
```

You don't have to provide all of the arguments for updating. You can provide as few as one up to all, but if you update the creators, you have to provide the complete set. For example if we just wanted to update the uri, we could just run.

```
cargo run -- metadata-update Cbg5o1tarienqQeQ8FcS6inGw2edrZ73znyYhVFtXa8b --uri ipfs://updated_uri
```

and then to see that just the uri has been updated.

```
cargo run -- metadata-info Cbg5o1tarienqQeQ8FcS6inGw2edrZ73znyYhVFtXa8b
```

```
Address: qHFGEW7vnW61Arupoo4WcV3uqoDiRRhFv3fzHGPusBt
Key: MetadataV1
Update Authority: 61mVTaw6hBtwWnSaGXRSJePFWEQqipeCka3evytEVNUp
Mint: Cbg5o1tarienqQeQ8FcS6inGw2edrZ73znyYhVFtXa8b
Primary Sale Happened: false
Is Mutable: true
Edition Nonce: 255
Name: My NFT
Symbol: NFT
Uri: ipfs://updated_uri
Seller Fee Basis Points: 0
Creators: 2
  [0] Address: 61mVTaw6hBtwWnSaGXRSJePFWEQqipeCka3evytEVNUp
      Verified: false
      Share: 50

  [1] Address: 7oHuVGKc5ZA2tdJX2xLxfUuZPf4RWMsEuNFWkByZNNs7
      Verified: false
      Share: 50
```

## Upload Files to Arweave

### Get and Arweave Wallet
The first thing you have to do is get some AR tokens, but efore you actually get tokens, you'll need a wallet to transfer them into. You can get a wallet directly from Arweave [here](https://faucet.arweave.net/). You will download a json file and this command line tool is set up to read from that file. You can save it whereever you like and then either provide the location as an argument to the commands `--keypair-path` or better yet, add the path to an environment variable named `ARWEAVE_KEYPAIR_PATH`.

### Purchase AR Tokens
Tokens can be purchased at either [gate.io](https://www.gate.io/) or [huobi.com](https://www.huobi.com/en-us/), or you can swap for them in [this Uniswap pool](https://info.uniswap.org/#/pools/0x3afec5673a547861877f4d722a594171595e561b). You likely won't need very many tokens since the cost of storage is relatively cheap. You can check to see how much storage costs in both Arweave tokens (AR) and USD by running the command below, which will give the cost of uploading 1 megabyte.

```
cargo run arweave price 1000000
```

```
The price to upload 1000000 bytes to arweave is 426163608 winstons ($0.023485877).
```

The price in AR is actually quoted in Winstons, of which there are 10^12 per AR. As of 2021-10-26, AR was trading for $55.10 USD and the cost of storage was $0.023 per megabyte.

### Upload a File

There is quite an involved process in correctly encoding files to be uploaded and written to the Arweave network. Fortunatley, all of that happens behind the scenes and all you have to do is provide the path of the file that you'd like to upload and the rest happens automatically. For example to upload a file named `0.png` in the same directory you are entering commands from, you would just enter

```
cargo run arweave upload-file 0.png
```

You will get back the id of the uploaded file and a notification that the upload transaction has been received. However, it can take some time before your file is actually written to the blockchain, so you need to come back and check the status of your upload later. You can do that by entering

```
cargo run arweave status <ID>
```

where id is the id you got back when you uploaded the file. Keep in mind that Arweave caches all of the files that it receives and just becuase it is visible at `https://arweave.net/<ID>` does not mean that it has been successfully written to the blockchain. You need to verify the status, either by runnng the status command or by visiting `https://arweave.net/tx/<ID>/status`.


## Todo
- [x] Upload to storage
- [x] Proper tests for arweave module
- [ ] Creator verification
- [ ] Add individual commands for minting tokens and creating master editions
- [ ] Display edition info
- [ ] Create and update from json files
- [ ] Integration tests
- [ ] Bulkify
- [ ] Vault
- [ ] Auction
- [ ] Candy Store
- [ ] Fractionalization
- [ ] Custom Edition Metadata
- [ ] Remote Wallet

## Implementation Details

This project has been separate into two related crates at this point, `metaplex_cli` and `arweave_rs`. There was enough complexity and standalone functionality in `arweave_rs` that it made sense to break it out separately.

### metaplex_cli
The command line interface includes output features and cli tooling from the [Solana token program cli](https://github.com/solana-labs/solana-program-library/tree/master/token/cli/src), including the ability to produce output for display or json, either normal or compact, and use default values from solana-cli local config. It also makes use of [solana-clap-utils](https://github.com/solana-labs/solana/tree/master/clap-utils) for efficient validation and argument parsing.

### arweave_rs
Includes standalone functionality to calculate merkle roots for chunked transactions, resolved their proofs and then validation transaction chunks. The current api of this application doesn't take advantage of the chunks api and instead using the `/tx` endpoint to upload files in a single network request. The work to create the chunks has been done and the testing just needs to be done to sort out its use. The current application also doesn't do an concurrent processesing, althought is developed in async. Although not critical for uploading relatively small batches of files for NFT projects, implementing chunking and concurrent processing should be quite performant.


