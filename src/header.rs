#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

use codec::{Decode, Encode};

pub type TXOutputHeader = u16;

#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Clone, Encode, Decode, Eq, PartialEq, PartialOrd, Ord, Hash, Debug)]
pub enum SignatureMethod {
    BLS,
    Schnorr,
    ZkSnark,
}

impl SignatureMethod {
    pub fn extract(header: TXOutputHeader) -> Result<SignatureMethod, &'static str> {
        Ok(match header & 7u16 {
            0u16 => SignatureMethod::BLS,
            1u16 => SignatureMethod::Schnorr,
            2u16 => SignatureMethod::ZkSnark,
            _ => {
                return Err("unsupported signature method");
            }
        })
    }
}

#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Clone, Encode, Decode, Eq, PartialEq, PartialOrd, Ord, Hash, Debug)]
pub enum TokenType {
    MLT,
    ETH,
    BTC,
}

impl TokenType {
    pub fn extract_for_value(header: TXOutputHeader) -> Result<TokenType, &'static str> {
        Ok(match header & 504u16 {
            0u16 => TokenType::MLT,
            8u16 => TokenType::ETH,
            16u16 => TokenType::BTC,
            _ => {
                return Err("unsupported token type");
            }
        })
    }

    pub fn extract_for_fee(header: TXOutputHeader) -> Result<TokenType, &'static str> {
        let fee_bits = header >> 6;
        Self::extract_for_value(fee_bits)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use frame_support::{assert_err, assert_ok};

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
        assert_err!(SignatureMethod::extract(x), "unsupported signature method");
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
        assert_err!(TokenType::extract_for_value(x), "unsupported token type");
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
        assert_err!(TokenType::extract_for_fee(x), "unsupported token type");
    }
}
