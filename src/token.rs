use common_access_token::{
    Algorithm, CborValue, KeyId, RegisteredClaims, TokenBuilder, cat_keys, catr, current_timestamp,
    token::MacType,
};
use hex::FromHex;
use std::collections::BTreeMap;

#[derive(clap::ValueEnum, Clone, Debug, PartialEq)]
pub enum TokenType {
    Header,
    Cookie,
    /// This is needed as a fix for airply to work with initial token
    /// as query and moved into cooke to be handled by the playing device
    CookieAsQuery,
}
fn decode_string(s: &str) -> Vec<u8> {
    let result = Vec::from_hex(s);
    match result {
        Ok(bytes) => bytes,
        Err(_) => panic!("Could not create byte key from string"),
    }
}

fn catr(variant: &TokenType, time: u64, ttl: u64, domain: &str) -> BTreeMap<i32, CborValue> {
    match variant {
        TokenType::Cookie | TokenType::CookieAsQuery => {
            let cookie_domain = format!("Domain={}", domain);
            catr::cookie_renewal(
                ttl as i64,
                Some((time + ttl / 2) as i64),
                Some("CTA-Common-Access-Token"),
                Some(vec![
                    "Secure",
                    "HttpOnly",
                    cookie_domain.as_str(),
                    "path=/",
                    "SameSite=None",
                ]),
            )
        }
        TokenType::Header => catr::header_renewal(
            ttl as i64,
            Some((time + ttl / 2) as i64),
            Some("CTA-Common-Access-Token"),
            Some(vec![]),
        ),
    }
}
// CBOR stands for Concise Binary Object Representation.
// json: { "temp": 22.5, "unit": "C" }
// cbor A2              # map of 2 pairs
//        64            # text string of length 4
//          74656D70    # "temp"
//        16            # unsigned int 22
//        64            # text string of length 4
//          756E6974    # "unit"
//        61            # text string of length 1
//          43          # "C"
//
// The first byte of any CBOR item is structured like this:
//
// 3 bits: Major type
// 5 bits: Additional information (often the value or length)
//
// A2 in binary is: 10100010
// 101 → Major type 5 (which means map)
// 00010 → Additional info 2 (which means 2 key-value pairs)
//
// COSE stands for CBOR Object Signing and Encryption.
pub fn create_token(
    key: &str,
    ttl: u64,
    token_type: &TokenType,
    domain: &str,
    issuer: &str,
) -> Vec<u8> {
    let key = decode_string(key);
    let now = current_timestamp();

    let token = TokenBuilder::new()
        .algorithm(Algorithm::HmacSha256)
        .unprotected_key_id(KeyId::string("Symmetric256"))
        .registered_claims(
            RegisteredClaims::new()
                .with_issuer(issuer)
                .with_subject("user_id:asset_id:session_id")
                .with_issued_at(now)
                .with_expiration(now + 2 * ttl)
                .with_cti(Vec::from([1, 2, 3, 4])),
        )
        .custom_cbor(
            cat_keys::CATR,
            catr::create(catr(&token_type, now, ttl, domain)),
        )
        .mac_type(MacType::MAC0(true))
        .use_cwt_tag(true)
        .sign(&key)
        .expect("Failed to sign token");

    token.to_bytes().expect("failed to encode token")
}
