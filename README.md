
# **Bitcoin Transaction Demo**

This project demonstrates creating and spending Bitcoin transactions using the `rust-bitcoin` library in regtest mode. It includes a Rust program that generates two transactions (one spending a coinbase UTXO and another spending the output of the first transaction) and submits them to a local bitcoind node via JSON-RPC. The setup runs in a Docker container for easy execution.

## **Prerequisites**

* Docker installed on your Linux system

## **Setup and Running**

**Clone the repository** (or create the project structure with the provided files):
  
	git clone https://github.com/Garvit-77/P2PoolV2-test.git  
	cd P2PoolV2-test

**Build and run the Docker container**:

	sudo docker build \-t test .  
	sudo docker run \-it \--rm \-p 18443:18443 test

**Observe the output**:

   * The container starts bitcoind in regtest mode and creates a legacy wallet.  
   * The Rust program executes, performing the following:  
     * Imports an address as watch-only.  
     * Mines 101 blocks to create a mature coinbase UTXO.  
     * Creates and submits a signed transaction spending the UTXO.  
     * Mines a block to confirm it.  
     * Creates and submits a second signed transaction spending the first transaction’s output.  
     * Mines another block to confirm it.  
   * The program prints the transaction IDs as they are submitted.

## **Project Structure**

* `Dockerfile`: Builds the Rust application and sets up bitcoind.  
* `bitcoin.conf`: Configuration for bitcoind in regtest mode.  
* `run.sh`: Script to start bitcoind, create a legacy wallet, and run the Rust program.  
* `rust-bitcoin-tx/`: Directory containing the Rust project.  
  * `Cargo.toml`: Rust dependencies.  
  * `src/main.rs`: Main Rust code for transaction creation and submission.
