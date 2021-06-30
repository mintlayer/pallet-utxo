#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
use sp_core::sp_std::convert::TryFrom;

use codec::{Decode, Encode};

pub type TXOutputHeader = u16;

// https://stackoverflow.com/posts/57578431/revisions from Shepmaster
// whenever a new type/variant is supported, we don't have to code a lot of 'matches' boilerplate.
macro_rules! u16_to_enum {
    ($(#[$meta:meta])* $vis:vis enum $name:ident {
        $($(#[$vmeta:meta])* $vname:ident $(= $val:expr)?,)*
    }) => {
        $(#[$meta])*
        $vis enum $name {
            $($(#[$vmeta])* $vname $(= $val)?,)*
        }

        impl TryFrom<u16> for $name {
            type Error = &'static str;

            fn try_from(v: u16) -> Result<Self, Self::Error> {
                match v {
                    $(x if x == $name::$vname as u16 => Ok($name::$vname),)*
                    _ => {
                        Err(stringify!(unsupported $name))
                    },
                }
            }
        }
    }
}

u16_to_enum! {
    #[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
    #[derive(Clone, Encode, Decode, Eq, PartialEq, PartialOrd, Ord, Hash, Debug)]
    pub enum SignatureMethod {
        BLS = 0,
        Schnorr = 1,
        ZkSnark = 2,
    }
}

impl SignatureMethod {
    pub fn extract(header: TXOutputHeader) -> Result<SignatureMethod, &'static str> {
        SignatureMethod::try_from(header & 7u16)
    }

    pub(crate) fn insert(
        header: &mut TXOutputHeader,
        signature_method: SignatureMethod,
    ) {
        *header = header.clone() & 0b1_111111_111111_000; // remove the original signature, if any.
        let signature_method = signature_method as u16;
        *header = header.clone() | signature_method;
    }
}

u16_to_enum! {
    #[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
    #[derive(Clone, Encode, Decode, Eq, PartialEq, PartialOrd, Ord, Hash, Debug)]
    pub enum TokenType {
        MLT = 0,
        ETH = 8,
        BTC = 16,
    }
}

impl TokenType {
    pub fn extract_for_value(header: TXOutputHeader) -> Result<TokenType, &'static str> {
        TokenType::try_from(header & 504u16)
    }

    pub fn extract_for_fee(header: TXOutputHeader) -> Result<TokenType, &'static str> {
        let fee_bits = header >> 6;
        Self::extract_for_value(fee_bits)
    }

    pub(crate) fn insert_value_type(
        header: &mut TXOutputHeader,
        token_type: TokenType,
    ) {
        *header = header.clone() & 0b1_111111_000000_111; // remove original token type, if any.
        let token_type = token_type as u16;
        *header = header.clone() | token_type;
    }

    pub(crate) fn insert_fee_type(header: &mut TXOutputHeader, token_type: TokenType) {
        *header = header.clone() & 0b1_000000_111111_111;
        let token_type = token_type as u16;
        let token_type = token_type * 64;
        *header = header.clone() | token_type;
    }
}

pub fn validate_header(header: TXOutputHeader) -> Result<(), &'static str> {
    SignatureMethod::extract(header)?;
    TokenType::extract_for_value(header)?;
    TokenType::extract_for_fee(header)?;

    Ok(())
}

pub trait TXOutputHeaderImpls {
    fn set_value_token_type(&mut self, value_token_type: TokenType);
    fn set_fee_token_type(&mut self, fee_token_type: TokenType);
    fn set_signature_method(&mut self, signature_method: SignatureMethod);

    fn extract_value_token_type(&self) -> Result<TokenType, &'static str>;
    fn extract_fee_token_type(&self) -> Result<TokenType, &'static str>;
    fn extract_signature_method(&self) -> Result<SignatureMethod, &'static str>;

    fn validate_header(&self) -> Result<(), &'static str>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use frame_support::{assert_err, assert_ok};

    #[test]
    fn validate() {
        let improper_sig_meth = 0b11111_011u16;
        assert_err!(
            validate_header(improper_sig_meth),
            "unsupported SignatureMethod"
        );

        let improper_v_token_type = 0b11000_000u16;
        assert_err!(
            validate_header(improper_v_token_type),
            "unsupported TokenType"
        );

        let improper_f_token_type = 0b11011_000001_001;
        assert_err!(
            validate_header(improper_f_token_type),
            "unsupported TokenType"
        );

        let proper_header = 0b10_000010_010u16;
        assert_ok!(validate_header(proper_header));

        let proper_header = 0b01_000001_000u16;
        assert_ok!(validate_header(proper_header));

        let proper_header = 0u16;
        assert_ok!(validate_header(proper_header));
    }

    #[test]
    fn signatures() {
        let x = 0b11011_000u16; // last 3 bits are 000, so signature should be 0 or BLS.
        let signature = SignatureMethod::extract(x);
        assert!(signature.is_ok());
        assert_eq!(signature.unwrap(), SignatureMethod::BLS);

        let x = 0b0000100_001; // last 3 bits are 001, so signature should be Schnorr
        assert_eq!(
            SignatureMethod::extract(x).unwrap(),
            SignatureMethod::Schnorr
        );

        let x = 0b111110_010; // last 3 bits are 010, so signature should be ZkSnark
        assert_eq!(
            SignatureMethod::extract(x).unwrap(),
            SignatureMethod::ZkSnark
        );

        let x = 0b10_111; // last 3 bits is are, and it's not yet supported.
        assert_err!(SignatureMethod::extract(x), "unsupported SignatureMethod");

        let mut header: TXOutputHeader = 185u16; // last 3 bits are 001. Convert to 000 for BLS.
        SignatureMethod::insert(&mut header, SignatureMethod::BLS);
        assert_eq!(header, 184);

        // last 3 bits of header are 000. Convert to 010 for ZkSnark.
        SignatureMethod::insert(&mut header, SignatureMethod::ZkSnark);
        assert_eq!(header, 186);
    }

    #[test]
    fn value_token_types() {
        let x = 0b1010_000000_110; // the middle 6 bits are 000000, so type is MLT.
        let value_type = TokenType::extract_for_value(x);
        assert!(value_type.is_ok());
        assert_eq!(value_type.unwrap(), TokenType::MLT);

        let x = 0b111_000001_011; // the middle 6 bits are 000001, so type is ETH.
        assert_eq!(TokenType::extract_for_value(x).unwrap(), TokenType::ETH);

        let x = 0b000010_101; // the first 6 bits are 000010, so type is BTC.
        assert_eq!(TokenType::extract_for_value(x).unwrap(), TokenType::BTC);

        let x = 3u16;
        assert_eq!(TokenType::extract_for_value(x).unwrap(), TokenType::MLT);

        let x = 0b110001_000;
        assert_err!(TokenType::extract_for_value(x), "unsupported TokenType");

        let mut improper_header = 321u16; // 101000_001, and must be converted to 10_001.
        TokenType::insert_value_type(&mut improper_header, TokenType::BTC);
        assert_eq!(improper_header, 17);

        improper_header = 178u16; // 10110_010, and must be converted to 000000_010 or 2.
        TokenType::insert_value_type(&mut improper_header, TokenType::MLT);
        assert_eq!(improper_header, 2);
    }

    #[test]
    fn fee_token_types() {
        let x = 0b110001_000;
        assert_eq!(TokenType::extract_for_fee(x).unwrap(), TokenType::MLT);

        let x = 0b001_000000_100; // extract 000001
        assert_eq!(TokenType::extract_for_fee(x).unwrap(), TokenType::ETH);

        let x = 0b000010_111110_001; // extract the first 6 bits
        assert_eq!(TokenType::extract_for_fee(x).unwrap(), TokenType::BTC);

        let x = 0b11_000000_111;
        assert_err!(TokenType::extract_for_fee(x), "unsupported TokenType");

        let mut header = 1033u16; //10_000001_001, and convert token to MLT, or 9.
        TokenType::insert_fee_type(&mut header, TokenType::MLT);
        assert_eq!(header, 9);

        let mut improper_header = 386u16; // 110000_010 convert to token ETH: 1_110000_010
        TokenType::insert_fee_type(&mut improper_header, TokenType::ETH);
        assert_eq!(improper_header, 898);
    }
}
