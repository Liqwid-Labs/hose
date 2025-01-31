use std::ops::Deref;

use minicbor::data::Tag;
use num::ToPrimitive;
use num_bigint::Sign;

/// A wrapper around `num_bigint::BigInt` that implements `minicbor::Encode` and `minicbor::Decode`.
/// You should not pass this around and rather use the `num_bigint::BigInt` type. This is only
/// here to make it possible to encode and decode BigInts in CBOR.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BigInt(num_bigint::BigInt);

impl Deref for BigInt {
    type Target = num_bigint::BigInt;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

macro_rules! impl_from_for_bigint {
    ($type:ty) => {
        impl From<$type> for BigInt {
            fn from(value: $type) -> Self {
                Self(num_bigint::BigInt::from(value))
            }
        }
    };
}

impl_from_for_bigint!(u64);
impl_from_for_bigint!(u32);
impl_from_for_bigint!(u16);
impl_from_for_bigint!(u8);

impl_from_for_bigint!(i64);
impl_from_for_bigint!(i32);
impl_from_for_bigint!(i16);
impl_from_for_bigint!(i8);

impl From<num_bigint::BigInt> for BigInt {
    fn from(value: num_bigint::BigInt) -> Self {
        Self(value)
    }
}

impl From<BigInt> for num_bigint::BigInt {
    fn from(val: BigInt) -> Self {
        val.0
    }
}

impl From<pallas::codec::utils::AnyUInt> for BigInt {
    fn from(value: pallas::codec::utils::AnyUInt) -> Self {
        match value {
            pallas::codec::utils::AnyUInt::U8(v) => Self(num_bigint::BigInt::from(v)),
            pallas::codec::utils::AnyUInt::U16(v) => Self(num_bigint::BigInt::from(v)),
            pallas::codec::utils::AnyUInt::U32(v) => Self(num_bigint::BigInt::from(v)),
            pallas::codec::utils::AnyUInt::U64(v) => Self(num_bigint::BigInt::from(v)),
            pallas::codec::utils::AnyUInt::MajorByte(v) => Self(num_bigint::BigInt::from(v)),
        }
    }
}

#[derive(Debug)]
pub enum IntoAnyUIntError {
    Overflow,
    Negative,
}

impl TryInto<pallas::codec::utils::AnyUInt> for BigInt {
    type Error = IntoAnyUIntError;

    fn try_into(self) -> Result<pallas::codec::utils::AnyUInt, Self::Error> {
        if self.sign() == Sign::Minus {
            Err(IntoAnyUIntError::Negative)
        } else if self.0 > u64::MAX.into() {
            Err(IntoAnyUIntError::Overflow)
        } else if self.0 > u32::MAX.into() {
            Ok(pallas::codec::utils::AnyUInt::U64(
                self.0.to_u64().expect("to_u64 should not fail"),
            ))
        } else if self.0 > u16::MAX.into() {
            Ok(pallas::codec::utils::AnyUInt::U32(
                self.0.to_u32().expect("to_u32 should not fail"),
            ))
        } else if self.0 > u8::MAX.into() {
            Ok(pallas::codec::utils::AnyUInt::U16(
                self.0.to_u16().expect("to_u16 should not fail"),
            ))
        } else if self.0 <= 0x17.into() {
            Ok(pallas::codec::utils::AnyUInt::MajorByte(
                self.0.to_u8().expect("to_u8 should not fail"),
            ))
        } else {
            Ok(pallas::codec::utils::AnyUInt::U8(
                self.0.to_u8().expect("to_u8 should not fail"),
            ))
        }
    }
}

impl<'b, C> minicbor::decode::Decode<'b, C> for BigInt {
    fn decode(
        d: &mut minicbor::Decoder<'b>,
        _ctx: &mut C,
    ) -> Result<Self, minicbor::decode::Error> {
        match d.datatype()? {
            minicbor::data::Type::U8 => Ok(BigInt(d.u8()?.into())),
            minicbor::data::Type::U16 => Ok(BigInt(d.u16()?.into())),
            minicbor::data::Type::U32 => Ok(BigInt(d.u32()?.into())),
            minicbor::data::Type::U64 => Ok(BigInt(d.u64()?.into())),
            minicbor::data::Type::I8 => Ok(BigInt(d.i8()?.into())),
            minicbor::data::Type::I16 => Ok(BigInt(d.i16()?.into())),
            minicbor::data::Type::I32 => Ok(BigInt(d.i32()?.into())),
            minicbor::data::Type::I64 => Ok(BigInt(d.i64()?.into())),
            minicbor::data::Type::Tag => {
                let tag = d.tag()?;
                match tag.as_u64() {
                    0x02 => {
                        let bs = d.bytes()?;
                        Ok(num_bigint::BigInt::from_bytes_be(Sign::Plus, bs).into())
                    }
                    0x03 => {
                        let bs = d.bytes()?;
                        Ok(num_bigint::BigInt::from_bytes_be(Sign::Minus, bs).into())
                    }
                    _ => Err(minicbor::decode::Error::message("invalid tag for BigInt")),
                }
            }
            _ => Err(minicbor::decode::Error::message(
                "invalid data type for BigInt",
            )),
        }
    }
}

impl<C> minicbor::encode::Encode<C> for BigInt {
    fn encode<W: minicbor::encode::Write>(
        &self,
        e: &mut minicbor::Encoder<W>,
        _ctx: &mut C,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        match TryInto::<pallas::codec::utils::AnyUInt>::try_into(self.clone()) {
            Ok(anyuint) => anyuint.encode(e, _ctx),
            Err(_) => match self.to_bytes_be() {
                (Sign::Plus, bs) => {
                    e.tag(Tag::new(0x02))?;
                    e.bytes(&bs)?;
                    Ok(())
                }
                (Sign::Minus, bs) => {
                    e.tag(Tag::new(0x03))?;
                    e.bytes(&bs)?;
                    Ok(())
                }
                (Sign::NoSign, _) => Err(minicbor::encode::Error::message(
                    "the impossible happened! no-sign, but we have covered non-big int cases.",
                )),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;

    #[test]
    fn test_from_large_uint() {
        let example_cbor = hex::decode("1B004195D11058B7E2").unwrap();
        let decoded: BigInt = minicbor::decode(&example_cbor).unwrap();
        assert_eq!(*decoded, num_bigint::BigInt::from(18460598641145826u64));
    }

    #[test]
    fn test_from_int() {
        let example_cbor = hex::decode("390055").unwrap();
        let decoded: BigInt = minicbor::decode(&example_cbor).unwrap();
        assert_eq!(*decoded, num_bigint::BigInt::from(-86i16));
    }

    #[test]
    fn test_to_biguint() {
        let bigint: BigInt = num_bigint::BigInt::from_str(
            "10000000000000000000000000000000000000000000000000000000000000000",
        )
        .expect("should be able to parse bigint")
        .into();

        let encoded = minicbor::to_vec(bigint.clone()).unwrap();

        assert_eq!(
            encoded,
            hex::decode("c2581b184f03e93ff9f4daa797ed6e38ed64bf6a1f010000000000000000").unwrap()
        );

        let decoded: BigInt = minicbor::decode(&encoded).unwrap();

        assert_eq!(decoded, bigint);
    }

    #[test]
    fn test_to_bignint() {
        let bigint: BigInt = num_bigint::BigInt::from_str(
            "-10000000000000000000000000000000000000000000000000000000000000000",
        )
        .expect("should be able to parse bigint")
        .into();

        let encoded = minicbor::to_vec(bigint.clone()).unwrap();

        assert_eq!(
            encoded,
            hex::decode("c3581b184f03e93ff9f4daa797ed6e38ed64bf6a1f010000000000000000").unwrap()
        );

        let decoded: BigInt = minicbor::decode(&encoded).unwrap();

        assert_eq!(decoded, bigint);
    }

    #[test]
    fn test_to_small_uint() {
        let bigint: BigInt = num_bigint::BigInt::from_str("5")
            .expect("should be able to parse bigint")
            .into();

        let encoded = minicbor::to_vec(bigint.clone()).unwrap();

        assert_eq!(hex::encode(encoded.clone()), "05");

        let decoded: BigInt = minicbor::decode(&encoded).unwrap();

        assert_eq!(decoded, bigint);
    }

    #[test]
    fn test_zero() {
        let bigint: BigInt = num_bigint::BigInt::from_str("0")
            .expect("should be able to parse bigint")
            .into();

        let encoded = minicbor::to_vec(bigint.clone()).unwrap();

        assert_eq!(hex::encode(encoded.clone()), "00");

        let decoded: BigInt = minicbor::decode(&encoded).unwrap();

        assert_eq!(decoded, bigint);
    }
}
