# µChain - Blockchain for prototyping
> Work in progress

µChain _pronounced_ `mu-chain`, is a blockchain base/framework for prototyping blockchain projects
## Quickstart

### 🧰 Install Dependencies
* **Rust Toolchain `Linux/MacOS`**
    ```shell
    curl https://sh.rustup.rs -sSf | sh
    ```
    ```shell
    rustup default nightly
    ```
* **Install gRPC, RockDB dependencies**

  **`Linux`**

  ```shell
  sudo apt install clang libclang-dev llvm llvm-dev linux-kernel-headers libev-dev
  ```
  ```shell
  sudo apt install cmake libprotobuf-dev protobuf-compiler
  ```
  **`MacOS`**

  ```shell
  brew install --with-toolchain llvm
  ```
  ```shell
  brew install cmake automake libtool
  ```
### ⬇️ Download

* Download the source code
    ```shell
    git clone https://github.com/mambisi/uchain
    ```
    ```shell
    cd uchain
    ```
### ⌛️ Running node `Linux/MacOS`
* Build the node from source
    ```shell
    cargo build --release --package node
    ```
* Generate Node Identity
    ```shell
    ./target/release/node identity generate
    ```
* Initialize node configuration
    ```shell
    ./target/release/node config init
    ```
* Create a miner account (optional - required if you want to run as a miner)
  ```shell
  ./target/release/node account new
  ```
  Output:
  ```json
  {
    "address": "0xa253d958f45db8aa712787cee1322aa2d7438a8f",
    "secret" : "0xd2e73c5bf670001803d9436a78d14ca9c12185f33fbc197274a104d817a088ab"
  }
  ```
  Set miner
  ```shell
  ./target/release/node config update --miner="0xa253d958f45db8aa712787cee1322aa2d7438a8f"
  ```
* Run node
  ```shell
  ./target/release/node run
  ```

### RPC Usage
µChain uses gRPC to interact with the node, user can use [bloomRPC](https://github.com/bloomrpc/bloomrpc.git) to interact with the blockchain
* **Protobuf files can be found** [here](/proto/schema)
* **Documentation**
  [RPC Documentation](/docs/rpc.md)
* **Clients**
  * Gui: [bloomRPC](https://github.com/bloomrpc/bloomrpc.git)
  * Cli: [gRPCurl](https://github.com/fullstorydev/grpcurl)
