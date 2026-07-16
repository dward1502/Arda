// sigil: SCROLL
use sodiumoxide::crypto::box_::{gen_keypair};
use base64::encode;

let (pubkey, seckey) = gen_keypair();
println!("WARDEN_PUBLIC_KEY_BASE64={}", encode(pubkey.as_ref()));
println!("WARDEN_SECRET_KEY_BASE64={}", encode(seckey.as_ref()));
