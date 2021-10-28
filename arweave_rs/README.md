
# arweave_rs

This crate includes functionality to upload files to the [Arweave](https://www.arweave.org/) permaweb from the command line.


## Upload Files to Arweave

### Get an Arweave Wallet
The first thing you have to do is get some AR tokens, but before you actually get tokens, you'll need a wallet to transfer them into. You can get a wallet directly from Arweave [here](https://faucet.arweave.net/). You will download a json file and the application is set up to read from that file. You can save it whereever you like and then either provide the location as an argument to the commands `--keypair-path` or better yet, add the path to an environment variable named `ARWEAVE_KEYPAIR_PATH`.

### Purchase AR Tokens
Tokens can be purchased at either [gate.io](https://www.gate.io/) or [huobi.com](https://www.huobi.com/en-us/), or you can swap for them in [this Uniswap pool](https://info.uniswap.org/#/pools/0x3afec5673a547861877f4d722a594171595e561b). You likely won't need very many tokens since the cost of storage is relatively cheap. You can check to see how much storage costs in both Arweave tokens (AR) and USD by running the command below, which will give the cost of uploading 1 megabyte. [Here](https://arweave.news/how-to-buy-arweave-token/) is an overview from Arweave on purchasing options from the U.S.

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
- [ ] Use chunking api and async to upload files faster
- [ ] Implement bulk uploads - up load all files in a directory
- [ ] Use a cache/manifest to keep track of ids and statuses during bulk uploads

## Implementation Details

Includes standalone functionality to calculate merkle roots for chunked transactions, resolve their proofs and validate chunks. The current api doesn't take advantage of the chunks api and instead is currently using the `/tx` endpoint. The work to create the chunks has been done and with only some minor additional testing, it will use the `/chunks` endpoint. The current application also doesn't do any concurrent processesing even though functions that would benefit from concurent processing have been developed as such. Although not critical for uploading relatively small batches of files for NFT projects, implementing chunking and concurrent processing will make the process of uploading files very fast.

## Resources

### Bundling

* [Bundles Will Take Over Arweave: A Look At the Permaweb's First L2](https://arweave.news/bundles/)
* [Bundlr Docs](https://docs.bundlr.network/network-overview)
* [arbundles](https://github.com/Bundlr-Network/arbundles/tree/master/src)
* [ANS-104](https://github.com/joshbenaron/arweave-standards/blob/ans104/ans/ANS-104.md)
* [https://bundlr.arweave.net/](https://bundlr.arweave.net/)
* [Bundlr discord channel](https://discord.com/channels/864852288002850866/865652381928259634)
