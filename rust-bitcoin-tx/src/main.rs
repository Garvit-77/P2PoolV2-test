use bitcoin::blockdata::script::{Builder, PushBytesBuf};
use bitcoin::blockdata::transaction::{Transaction, TxIn, TxOut, OutPoint};
use bitcoin::blockdata::witness::Witness;
use bitcoin::consensus::encode::serialize_hex;
use bitcoin::hash_types::Txid;
use bitcoin::network::Network;
use bitcoin::secp256k1::{Secp256k1, Message};
use bitcoin::sighash::{SighashCache, EcdsaSighashType};
use bitcoin::{Address, ScriptBuf, PrivateKey, Amount};
use reqwest::blocking::Client;
use serde_json::{json, Value};
use std::str::FromStr;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new();
    let rpc_url = "http://localhost:18443/";
    let rpc_user = "user";
    let rpc_pass = "pass";

    // Initialize secp256k1 context
    let secp = Secp256k1::new();

    // Use hardcoded private key and derive address
    let private_key = PrivateKey::from_wif("cVt4o7BGAig1UXywgGSmARhxMdzP5qvQsxKkSsc1XEkw3tDTQFpy")?;
    let public_key = private_key.public_key(&secp);
    let address = Address::p2pkh(&public_key, Network::Regtest);
    let address_str = address.to_string();
    println!("Address: {}", address_str);

    // Import address to wallet as watch-only
    import_address(&client, rpc_url, rpc_user, rpc_pass, &address_str)?;

    // Generate 101 blocks to address to create mature coinbase UTXO
    generate_blocks(&client, rpc_url, rpc_user, rpc_pass, 101, &address_str)?;

    // Retrieve UTXO for the address
    let utxo = get_utxo_for_address(&client, rpc_url, rpc_user, rpc_pass, &address_str)?;
    println!("Found UTXO: {:?}", utxo);

    // Create and sign first transaction
    let tx1 = create_first_tx(&utxo, &address, &secp, &private_key)?;
    let tx1_hex = serialize_hex(&tx1);
    let tx1_id = submit_transaction(&client, rpc_url, rpc_user, rpc_pass, &tx1_hex)?;
    println!("First transaction submitted: {}", tx1_id);

    // Mine 1 block to confirm first transaction
    generate_blocks(&client, rpc_url, rpc_user, rpc_pass, 1, &address_str)?;

    // Get amount from first transaction's output
    let tx1_amount = tx1.output[0].value.to_sat();

    // Create and sign second transaction
    let tx2 = create_second_tx(&tx1_id, &address, &secp, &private_key, tx1_amount)?;
    let tx2_hex = serialize_hex(&tx2);
    let tx2_id = submit_transaction(&client, rpc_url, rpc_user, rpc_pass, &tx2_hex)?;
    println!("Second transaction submitted: {}", tx2_id);

    // Mine 1 block to confirm second transaction
    generate_blocks(&client, rpc_url, rpc_user, rpc_pass, 1, &address_str)?;

    Ok(())
}

fn import_address(client: &Client, rpc_url: &str, user: &str, pass: &str, address: &str) -> Result<(), Box<dyn std::error::Error>> {
    let request = json!({
        "jsonrpc": "2.0",
        "method": "importaddress",
        "params": [address, "", false],
        "id": 1
    });

    client.post(rpc_url)
        .basic_auth(user, Some(pass))
        .json(&request)
        .send()?
        .json::<Value>()?;

    Ok(())
}

fn generate_blocks(client: &Client, rpc_url: &str, user: &str, pass: &str, num_blocks: u32, address: &str) -> Result<(), Box<dyn std::error::Error>> {
    let request = json!({
        "jsonrpc": "2.0",
        "method": "generatetoaddress",
        "params": [num_blocks, address],
        "id": 2
    });

    client.post(rpc_url)
        .basic_auth(user, Some(pass))
        .json(&request)
        .send()?
        .json::<Value>()?;

    Ok(())
}

fn get_utxo_for_address(client: &Client, rpc_url: &str, user: &str, pass: &str, address: &str) -> Result<(OutPoint, u64, ScriptBuf), Box<dyn std::error::Error>> {
    let request = json!({
        "jsonrpc": "2.0",
        "method": "listunspent",
        "params": [0, 9999999, [address]],
        "id": 3
    });

    let response = client.post(rpc_url)
        .basic_auth(user, Some(pass))
        .json(&request)
        .send()?
        .json::<Value>()?;

    let utxos = response["result"].as_array().ok_or("No UTXOs found")?;
    if utxos.is_empty() {
        return Err("No UTXOs found for address".into());
    }

    let utxo = &utxos[0];
    let txid = Txid::from_str(utxo["txid"].as_str().unwrap())?;
    let vout = utxo["vout"].as_u64().unwrap() as u32;
    let amount = (utxo["amount"].as_f64().unwrap() * 100_000_000.0) as u64;
    let script_pubkey = ScriptBuf::from_hex(utxo["scriptPubKey"].as_str().unwrap())?;

    Ok((OutPoint { txid, vout }, amount, script_pubkey))
}

fn create_first_tx(
    utxo: &(OutPoint, u64, ScriptBuf),
    to_address: &Address,
    secp: &Secp256k1<bitcoin::secp256k1::All>,
    private_key: &PrivateKey,
) -> Result<Transaction, Box<dyn std::error::Error>> {
    let amount = utxo.1 - 1000; // Subtract fee
    let mut tx = Transaction {
        version: bitcoin::transaction::Version::TWO,
        lock_time: bitcoin::absolute::LockTime::ZERO,
        input: vec![TxIn {
            previous_output: utxo.0,
            script_sig: ScriptBuf::new(),
            sequence: bitcoin::Sequence::MAX,
            witness: Witness::new(),
        }],
        output: vec![TxOut {
            value: Amount::from_sat(amount),
            script_pubkey: to_address.script_pubkey(),
        }],
    };

    // Sign the input
    let sighash = SighashCache::new(&tx).legacy_signature_hash(0, &utxo.2, EcdsaSighashType::All as u32)?;
    let message = Message::from(sighash);
    let signature = secp.sign_ecdsa(&message, &private_key.inner);
    let public_key = private_key.public_key(secp);

    let mut sig_with_type = signature.serialize_der().to_vec();
    sig_with_type.push(EcdsaSighashType::All as u8); // Append sighash type (0x01 for SIGHASH_ALL)
    let signature_push = PushBytesBuf::try_from(sig_with_type).expect("Signature too large");
    let pubkey_push = PushBytesBuf::try_from(public_key.to_bytes()).expect("Public key too large");

    let script_sig = Builder::new()
        .push_slice(&signature_push)
        .push_slice(&pubkey_push)
        .into_script();

    tx.input[0].script_sig = script_sig;

    Ok(tx)
}

fn create_second_tx(
    tx1_id: &str,
    to_address: &Address,
    secp: &Secp256k1<bitcoin::secp256k1::All>,
    private_key: &PrivateKey,
    prev_amount: u64,
) -> Result<Transaction, Box<dyn std::error::Error>> {
    let txid = Txid::from_str(tx1_id)?;
    let amount = prev_amount - 1000; // Subtract fee
    let mut tx = Transaction {
        version: bitcoin::transaction::Version::TWO,
        lock_time: bitcoin::absolute::LockTime::ZERO,
        input: vec![TxIn {
            previous_output: OutPoint { txid, vout: 0 },
            script_sig: ScriptBuf::new(),
            sequence: bitcoin::Sequence::MAX,
            witness: Witness::new(),
        }],
        output: vec![TxOut {
            value: Amount::from_sat(amount),
            script_pubkey: to_address.script_pubkey(),
        }],
    };

    // Sign the input
    let script_code = to_address.script_pubkey();
    let sighash = SighashCache::new(&tx).legacy_signature_hash(0, &script_code, EcdsaSighashType::All as u32)?;
    let message = Message::from(sighash);
    let signature = secp.sign_ecdsa(&message, &private_key.inner);
    let public_key = private_key.public_key(secp);

    let mut sig_with_type = signature.serialize_der().to_vec();
    sig_with_type.push(EcdsaSighashType::All as u8); // Append sighash type (0x01 for SIGHASH_ALL)
    let signature_push = PushBytesBuf::try_from(sig_with_type).expect("Signature too large");
    let pubkey_push = PushBytesBuf::try_from(public_key.to_bytes()).expect("Public key too large");

    let script_sig = Builder::new()
        .push_slice(&signature_push)
        .push_slice(&pubkey_push)
        .into_script();

    tx.input[0].script_sig = script_sig;

    Ok(tx)
}

fn submit_transaction(client: &Client, rpc_url: &str, user: &str, pass: &str, tx_hex: &str) -> Result<String, Box<dyn std::error::Error>> {
    let request = json!({
        "jsonrpc": "2.0",
        "method": "sendrawtransaction",
        "params": [tx_hex],
        "id": 4
    });

    let response = client.post(rpc_url)
        .basic_auth(user, Some(pass))
        .json(&request)
        .send()?
        .json::<Value>()?;

    let txid = response["result"].as_str().ok_or("Failed to get txid")?.to_string();
    Ok(txid)
}