use libp2p::core::PublicKey;

fn encode_pubk(pubk: PublicKey, n_bytes: Option<usize>) -> Result<String, String> {
    let pubk_u8s = pubk.into_protobuf_encoding();
    if let Some(n) = n_bytes {
        if n != pubk_u8s.len() {
            return Err(format!("Unexpected number of bytes in public key. Expected: {} , Actual: {}", n, pubk_u8s.len()))
        }
    }
    Ok(hex::encode(pubk_u8s))
}
fn decode_pubk(pubk_hex: String, n_bytes : Option<usize>) -> Result<PublicKey, String>{
    let pubk_u8s: Vec<u8> = match hex::decode(pubk_hex) {
        Ok (pubk_u8s) => pubk_u8s,
        Err (e) => {
            return Err(format!("Couldn't decode public key from hex-string to Vec<u8>: {}", e))
        }
    };
    if let Some(n) = n_bytes {
        if n != pubk_u8s.len() {
            return Err(format!("Unexpected number of bytes in public key. Expected: {} , Actual: {}", n, pubk_u8s.len()))
        }
    }
    match PublicKey::from_protobuf_encoding(pubk_u8s.as_slice()) {
        Ok(pubk) => Ok(pubk),
        Err(e) => {
            Err(format!("Couldn't decode public key from &[u8] to PublicKey: {}", e))
        }
    }
}
fn encode_hash(hash_u8s : Vec<u8>, n_bytes: Option<usize>) -> Result<String, String> {
    if let Some(n) = n_bytes {
        if n != hash_u8s.len() {
            return Err(format!("Unexpected number of bytes in hash. Expected: {} , Actual: {}", n, hash_u8s.len()))
        }
    }
    Ok(hex::encode(hash_u8s))
}
fn decode_hash(hash_hex: String, n_bytes: Option<usize>) -> Result<Vec<u8>, String>{
    let hash_u8s: Vec<u8> = match hex::decode(hash_hex) {
        Ok (pubk_u8s) => pubk_u8s,
        Err (e) => {
            return Err(format!("Couldn't decode hash from hex-string to Vec<u8>: {}", e))
        }
    };
    if let Some(n) = n_bytes {
        if n != hash_u8s.len() {
            return Err(format!("Unexpected number of bytes in hash. Expected: {} , Actual: {}", n, hash_u8s.len()))
        }
    }
    Ok(hash_u8s)
}
