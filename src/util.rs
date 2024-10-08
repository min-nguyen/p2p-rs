use libp2p::core::PublicKey;

pub const ZERO_U32 : [u8; 32] = [0; 32];
pub const ZERO_U64 : [u8; 64] = [0; 64];

pub fn encode_pubk(pubk: PublicKey) -> String {
    hex::encode(pubk.into_protobuf_encoding())
}
pub fn decode_pubk(pubk_hex: &String, n_bytes : usize) -> Result<PublicKey, String>{
    let pubk_u8s: Vec<u8> = decode_hex(&pubk_hex, n_bytes)?;
    match PublicKey::from_protobuf_encoding(pubk_u8s.as_slice()) {
        Ok(pubk) => Ok(pubk),
        Err(e) => {
            Err(format!("Couldn't decode public key from &[u8] to PublicKey: {}", e))
        }
    }
}
pub fn encode_hex<T : AsRef<[u8]>>(hash_u8s : T) -> String {
    hex::encode(hash_u8s)
}
pub fn decode_hex(hash_hex: &String, n_bytes : usize) -> Result<Vec<u8>, String>{
    let hash_u8s: Vec<u8> = match hex::decode(hash_hex) {
        Ok (pubk_u8s) => pubk_u8s,
        Err (e) => {
            return Err(format!("Couldn't decode from hex-string to Vec<u8>: {}", e))
        }
    };
    if hash_u8s.as_slice().len() != n_bytes {
        return Err(format!("Unexpected number of bytes in hex-string. Expected: {}, Got: {}", n_bytes, hash_u8s.as_slice().len()))
    }
    Ok(hash_u8s)
}