use libp2p::core::PublicKey;
use std::fmt;
pub const ZERO_U32 : [u8; 32] = [0; 32];
pub const ZERO_U64 : [u8; 64] = [0; 64];

#[derive(Debug)]
pub enum HexDecodeErr {
    ToPubk {
        msg: String
    },
    ToBytes {
        msg: String
    }
}

// Implementing the Display trait for HexDecodeErr
impl fmt::Display for HexDecodeErr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HexDecodeErr::ToPubk { msg } => {
                write!(f, "Hex Decode Error to Public Key: {}", msg)
            }
            HexDecodeErr::ToBytes { msg } => {
                write!(f, "Hex Decode Error to Bytes: {}", msg)
            }
        }
    }
}

pub fn encode_pubk_to_hex(pubk: PublicKey) -> String {
    hex::encode(pubk.into_protobuf_encoding())
}
pub fn decode_hex_to_pubk(pubk_hex: &String, n_bytes : usize) -> Result<PublicKey, HexDecodeErr>{
    let pubk_u8s: Vec<u8> = decode_hex_to_bytes(&pubk_hex, n_bytes)?;
    match PublicKey::from_protobuf_encoding(pubk_u8s.as_slice()) {
        Ok(pubk) => Ok(pubk),
        Err(e) => {
            Err(HexDecodeErr::ToPubk{msg : format!("{:?}", e)})
        }
    }
}
pub fn encode_bytes_to_hex<T : AsRef<[u8]>>(hash_u8s : T) -> String {
    hex::encode(hash_u8s)
}

pub fn decode_hex_to_bytes(hash_hex: &String, n_bytes : usize) -> Result<Vec<u8>, HexDecodeErr>{
    let hash_u8s: Vec<u8> = match hex::decode(hash_hex) {
        Ok (pubk_u8s) => pubk_u8s,
        Err (e) => {
            return Err(HexDecodeErr::ToBytes{msg : format!("{:?}", e)})
        }
    };
    if hash_u8s.as_slice().len() != n_bytes {
        return Err(HexDecodeErr::ToBytes{
            msg : format!("Unexpected number of bytes in hex-string. Expected: {}, Got: {}", n_bytes, hash_u8s.as_slice().len() )
        })
    }
    Ok(hash_u8s)
}

pub fn debug<T:std::fmt::Debug>(f : T) -> T{
    eprintln!("{:?}", f);
    f
}

pub fn pretty_hex(hex : &String) -> String {
    let mut s = hex.clone();
    s.truncate(20);
    s.push_str("...");
    s
}