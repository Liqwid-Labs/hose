use std::ops::Deref;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AikenOption<T>(Option<T>);

impl<T> Deref for AikenOption<T> {
    type Target = Option<T>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> From<Option<T>> for AikenOption<T> {
    fn from(v: Option<T>) -> Self {
        Self(v)
    }
}

impl<T> From<AikenOption<T>> for Option<T> {
    fn from(val: AikenOption<T>) -> Self {
        val.0
    }
}

impl<C, T: minicbor::encode::Encode<C>> minicbor::encode::Encode<C> for AikenOption<T> {
    // Required method
    fn encode<W: minicbor::encode::Write>(
        &self,
        e: &mut minicbor::Encoder<W>,
        ctx: &mut C,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        match &self.0 {
            None => {
                _ = e.tag(minicbor::data::Tag::new(121));
                _ = e.begin_array();
                _ = e.end();
                Ok(())
            }
            Some(v) => {
                _ = e.tag(minicbor::data::Tag::new(122));
                _ = e.begin_array();
                _ = v.encode(e, ctx);
                _ = e.end();
                Ok(())
            }
        }
    }
}

impl<'b, C, T: minicbor::decode::Decode<'b, C>> minicbor::decode::Decode<'b, C> for AikenOption<T> {
    fn decode(
        d: &mut minicbor::decode::Decoder<'b>,
        ctx: &mut C,
    ) -> Result<Self, minicbor::decode::Error> {
        let tag = d.tag()?;
        match tag.as_u64() {
            121 => Ok(None.into()),
            122 => {
                _ = d.array();
                let inner = <T>::decode(d, ctx)?;
                _ = d.skip();
                Ok(Some(inner).into())
            }
            _ => Err(minicbor::decode::Error::message(
                "Invalid tag for AikenOption",
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::bigint::BigInt;

    use super::*;

    #[test]
    fn test_aiken_option() {
        let v = AikenOption::from(Some(BigInt::from(1)));
        let encoded = minicbor::to_vec(&v).unwrap();

        let encoded_hex = hex::encode(encoded.clone());

        let expected_hex = "d87a9f01ff";

        assert_eq!(encoded_hex, expected_hex);

        let decoded = minicbor::decode::<AikenOption<BigInt>>(&encoded).unwrap();

        assert_eq!(decoded, AikenOption::from(Some(BigInt::from(1))));
    }
}
