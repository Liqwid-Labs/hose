use std::collections::HashMap;
use std::ops::Deref;
use std::str::FromStr;

use num::BigRational;
use serde::{Deserialize, Deserializer, Serialize};

#[derive(Debug, Clone, Serialize)]
pub struct RpcRequest<T: Serialize> {
    pub jsonrpc: String,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<T>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum RpcResponse<T, E> {
    Success(RpcSuccess<T>),
    Error(RpcError<E>),
}

impl<T, E> Into<Result<T, E>> for RpcResponse<T, E> {
    fn into(self) -> Result<T, E> {
        match self {
            RpcResponse::Success(success) => Ok(success.result),
            RpcResponse::Error(error) => Err(error.error),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct RpcSuccess<T> {
    pub jsonrpc: String,
    pub method: String,
    pub result: T,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RpcError<T> {
    pub jsonrpc: String,
    pub method: String,
    pub error: T,
}

#[derive(Debug, Clone, Serialize)]
pub struct TxCbor {
    /// A hex-encoded CBOR value
    pub cbor: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TxPointer {
    /// 32-byte hex-encoded blake2b digest of the transaction body
    pub id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TxOutputPointer {
    pub transaction: TxPointer,
    pub index: u32,
}

impl From<hydrant::primitives::TxOutputPointer> for TxOutputPointer {
    fn from(ptr: hydrant::primitives::TxOutputPointer) -> Self {
        Self {
            transaction: TxPointer {
                id: hex::encode(*ptr.hash),
            },
            index: ptr.index as u32,
        }
    }
}
impl From<&hydrant::primitives::TxOutputPointer> for TxOutputPointer {
    fn from(ptr: &hydrant::primitives::TxOutputPointer) -> Self {
        Self {
            transaction: TxPointer {
                id: hex::encode(*ptr.hash),
            },
            index: ptr.index as u32,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TxOutput {
    /// A Cardano address (either legacy format or new format)
    /// New: `addr1q9d34spgg2kdy47n82e7x9pdd6vql6d2engxmpj20jmhuc2047yqd4xnh7u6u5jp4t0q3fkxzckph4tgnzvamlu7k5psuahzcp`
    /// Legacy: `DdzFFzCqrht8mbSTZHqpM2u4HeND2mdspsaBhdQ1BowPJBMzbDeBMeKgqdoKqo1D4sdPusEdZJVrFJRBBxX1jUEofNDYCJSZLg8MkyCE`
    address: String,
    value: Balance,
    /// A Blake2b 32-byte hash digest, hex-encoded
    datum_hash: Option<String>,
    /// Hex-encoded CBOR value
    datum: Option<String>,
    // TODO: script
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionUnits {
    pub memory: Ratio,
    pub cpu: Ratio,
}

#[derive(Debug, Clone)]
pub struct Ratio(pub num_rational::BigRational);

impl Into<BigRational> for Ratio {
    fn into(self) -> BigRational {
        self.0
    }
}

impl<'de> serde::Deserialize<'de> for Ratio {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s: String = Deserialize::deserialize(deserializer)?;
        Ok(Ratio(
            num_rational::BigRational::from_str(&s)
                .map_err(|e| serde::de::Error::custom(e.to_string()))?,
        ))
    }
}

impl Serialize for Ratio {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.0.to_string())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedeemerPointer {
    pub purpose: RedeemerPurpose,
    pub index: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RedeemerPurpose {
    #[serde(rename = "spend")]
    Spend,
    #[serde(rename = "mint")]
    Mint,
    #[serde(rename = "publish")]
    Publish,
    #[serde(rename = "withdraw")]
    Withdraw,
    #[serde(rename = "vote")]
    Vote,
    #[serde(rename = "propose")]
    Propose,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Era {
    #[serde(rename = "byron")]
    Byron,
    #[serde(rename = "shelley")]
    Shelley,
    #[serde(rename = "allegra")]
    Allegra,
    #[serde(rename = "mary")]
    Mary,
    #[serde(rename = "alonzo")]
    Alonzo,
    #[serde(rename = "babbage")]
    Babbage,
    #[serde(rename = "conway")]
    Conway,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Language {
    #[serde(rename = "plutus:v1")]
    PlutusV1,
    #[serde(rename = "plutus:v2")]
    PlutusV2,
    #[serde(rename = "plutus:v3")]
    PlutusV3,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
#[serde(rename_all = "camelCase")]
pub enum InputSource {
    Inputs,
    Collaterals,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
#[serde(rename_all = "camelCase")]
pub enum CredentialOrigin {
    VerificationKey,
    Script,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidityInterval {
    pub invalid_before: Option<u64>,
    pub invalid_hereafter: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NumberOfBytes {
    pub bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProtocolVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: Option<u32>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TxId {
    /// Hex-encoded 32-byte blake2b hash digest
    pub id: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct StakePoolId {
    /// Hex-encoded 28-byte blake2b hash digest (pool1...)
    pub id: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct Balance {
    pub lovelace: u64,
    pub assets: Assets,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct Assets(HashMap<String, HashMap<String, u64>>);

impl Deref for Assets {
    type Target = HashMap<String, HashMap<String, u64>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'de> Deserialize<'de> for Balance {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let mut assets: HashMap<String, HashMap<String, u64>> = HashMap::deserialize(deserializer)?;

        // Require "ada.lovelace" entry to exist
        let lovelace = *assets
            .get("ada")
            .ok_or_else(|| serde::de::Error::missing_field("ada"))?
            .get("lovelace")
            .ok_or_else(|| serde::de::Error::missing_field("ada.lovelace"))?;
        assets.remove("ada");

        Ok(Balance {
            lovelace,
            assets: Assets(assets),
        })
    }
}

#[derive(Debug, Clone)]
pub struct AdaBalance {
    pub lovelace: u64,
}
impl<'de> Deserialize<'de> for AdaBalance {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let map: HashMap<String, HashMap<String, u64>> = HashMap::deserialize(deserializer)?;

        let ada_map = map
            .get("ada")
            .ok_or_else(|| serde::de::Error::missing_field("ada"))?;

        let lovelace = *ada_map
            .get("lovelace")
            .ok_or_else(|| serde::de::Error::missing_field("ada.lovelace"))?;

        Ok(AdaBalance { lovelace })
    }
}

#[derive(Debug, Clone)]
pub struct AdaBalanceDelta {
    pub lovelace: i64,
}

impl<'de> Deserialize<'de> for AdaBalanceDelta {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let map: HashMap<String, HashMap<String, i64>> = HashMap::deserialize(deserializer)?;

        let ada_map = map
            .get("ada")
            .ok_or_else(|| serde::de::Error::missing_field("ada"))?;

        let lovelace = *ada_map
            .get("lovelace")
            .ok_or_else(|| serde::de::Error::missing_field("ada.lovelace"))?;

        Ok(AdaBalanceDelta { lovelace })
    }
}

/// Helper macro for generating deserializable error types
#[macro_export]
macro_rules! define_ogmios_error {
    (
        $(#[$enum_meta:meta])*
        $vis:vis enum $enum_name:ident {
            $(
                $(#[$variant_meta:meta])*
                $code:literal => $variant:ident $({
                    $(
                        $(#[$field_meta:meta])*
                        $field:ident: $ty:ty
                    ),* $(,)?
                })?
                $(( $single_ty:ty ))?
            ),+
            $(,)?
            $(#[$fallback_meta:meta])*
            _ => $fallback_variant:ident { error: Value }
        }
    ) => {
        $(#[$enum_meta])*
        $vis enum $enum_name {
            $(
                $(#[$variant_meta])*
                $variant {
                    message: String,
                    $($(
                        $(#[$field_meta])*
                        $field: $ty,
                    )*)?
                    $(data: $single_ty,)?
                },
            )+
            $(#[$fallback_meta])*
            $fallback_variant {
                message: String,
                code: i32,
                error: serde_json::Value,
            },
        }

        impl $enum_name {
            pub fn code(&self) -> i32 {
                match self {
                    $(
                        $enum_name::$variant { .. } => $code,
                    )+
                    $enum_name::$fallback_variant { code, .. } => *code,
                }
            }

            pub fn message(&self) -> &str {
                match self {
                    $(
                        $enum_name::$variant { message, .. } => message,
                    )+
                    $enum_name::$fallback_variant { message, .. } => message,
                }
            }
        }

        impl<'de> serde::Deserialize<'de> for $enum_name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                #[derive(serde::Deserialize)]
                #[serde(rename_all = "camelCase")]
                struct RawError {
                    code: i32,
                    message: String,
                    data: Option<serde_json::Value>,
                }

                fn snake_to_camel(s: &str) -> String {
                    let mut result = String::with_capacity(s.len());
                    let mut capitalize_next = false;
                    for c in s.chars() {
                        if c == '_' {
                            capitalize_next = true;
                        } else if capitalize_next {
                            result.push(c.to_ascii_uppercase());
                            capitalize_next = false;
                        } else {
                            result.push(c);
                        }
                    }
                    result
                }

                fn get_field<T, E>(data: &serde_json::Value, snake_case_name: &str) -> Result<T, E>
                where
                    T: serde::de::DeserializeOwned,
                    E: serde::de::Error,
                {
                    let camel_case_name = snake_to_camel(snake_case_name);
                    data.get(&camel_case_name)
                        .ok_or_else(|| E::missing_field("tmp"))
                        .and_then(|v| serde_json::from_value(v.clone()).map_err(E::custom))
                }

                let raw = RawError::deserialize(deserializer)?;
                let data = raw.data;
                let message = raw.message;
                let code = raw.code;

                match code {
                    $(
                        $code => {
                            define_ogmios_error!(@deserialize_variant
                                $enum_name, $variant, message, data
                                $({ $($field: $ty),* })?
                                $(( $single_ty ))?
                            )
                        }
                    )+
                    _ => {
                        let error = data.unwrap_or(serde_json::Value::Null);
                        Ok($enum_name::$fallback_variant { message, code, error })
                    }
                }
            }
        }

        impl std::fmt::Display for $enum_name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "[{}] {}", self.code(), self.message())
            }
        }

        impl std::error::Error for $enum_name {}
    };

    // Internal rule: no data field
    (@deserialize_variant
        $enum_name:ident, $variant:ident, $message:ident, $data:ident
    ) => {
        Ok($enum_name::$variant { message: $message })
    };

    // Internal rule: struct-like with named fields
    (@deserialize_variant
        $enum_name:ident, $variant:ident, $message:ident, $data:ident
        { $($field:ident: $ty:ty),* }
    ) => {{
        let data = $data.ok_or_else(|| serde::de::Error::missing_field("data"))?;
        $(
            let $field: $ty = get_field(&data, stringify!($field))?;
        )*
        Ok($enum_name::$variant { message: $message, $($field),* })
    }};

    // Internal rule: single value
    (@deserialize_variant
        $enum_name:ident, $variant:ident, $message:ident, $data:ident
        ( $single_ty:ty )
    ) => {{
        let data = $data.ok_or_else(|| serde::de::Error::missing_field("data"))?;
        let data: $single_ty = serde_json::from_value(data).map_err(serde::de::Error::custom)?;
        Ok($enum_name::$variant { message: $message, data })
    }};
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;
    use serde_json::{Value, json};

    #[derive(Debug, Clone, PartialEq, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct CustomErrorData {
        details: String,
        severity: u32,
    }

    define_ogmios_error! {
        #[derive(Debug, Clone, PartialEq)]
        pub enum EvaluationError {
            1 => IncompatibleEra {
                incompatible_era: String,
            },
            2 => NodeTipTooOld {
                minimum_required_era: String,
                current_node_era: String,
            },
            3 => NoData,
            4 => SingleValue(CustomErrorData),
            _ => Unknown { error: Value }
        }
    }

    mod struct_variants {
        use super::*;

        #[test]
        fn deserialize_single_field() {
            let json = json!({
                "code": 1,
                "message": "Era mismatch",
                "data": {
                    "incompatibleEra": "Byron"
                }
            });

            let error: EvaluationError = serde_json::from_value(json).unwrap();

            assert_eq!(error.code(), 1);
            assert_eq!(error.message(), "Era mismatch");
            assert_eq!(
                error,
                EvaluationError::IncompatibleEra {
                    message: "Era mismatch".to_string(),
                    incompatible_era: "Byron".to_string(),
                }
            );
        }
    }

    mod no_data_variant {
        use super::*;

        #[test]
        fn deserialize_without_data_field() {
            let json = json!({
                "code": 3,
                "message": "No data error"
            });

            let error: EvaluationError = serde_json::from_value(json).unwrap();

            assert_eq!(error.code(), 3);
            assert_eq!(error.message(), "No data error");
            assert_eq!(
                error,
                EvaluationError::NoData {
                    message: "No data error".to_string(),
                }
            );
        }

        #[test]
        fn deserialize_with_null_data_field() {
            let json = json!({
                "code": 3,
                "message": "No data error",
                "data": null
            });

            let error: EvaluationError = serde_json::from_value(json).unwrap();

            assert_eq!(
                error,
                EvaluationError::NoData {
                    message: "No data error".to_string(),
                }
            );
        }
    }

    mod single_value_variant {
        use super::*;

        #[test]
        fn deserialize_single_value() {
            let json = json!({
                "code": 4,
                "message": "Custom error",
                "data": {
                    "details": "Something went wrong",
                    "severity": 5
                }
            });

            let error: EvaluationError = serde_json::from_value(json).unwrap();

            assert_eq!(error.code(), 4);
            assert_eq!(error.message(), "Custom error");
            assert_eq!(
                error,
                EvaluationError::SingleValue {
                    message: "Custom error".to_string(),
                    data: CustomErrorData {
                        details: "Something went wrong".to_string(),
                        severity: 5,
                    },
                }
            );
        }
    }

    mod unknown_variant {
        use super::*;

        #[test]
        fn deserialize_unknown_code_with_data() {
            let json = json!({
                "code": 9999,
                "message": "Unknown error",
                "data": {
                    "foo": "bar",
                    "baz": 123
                }
            });

            let error: EvaluationError = serde_json::from_value(json).unwrap();

            assert_eq!(error.code(), 9999);
            assert_eq!(error.message(), "Unknown error");
            match error {
                EvaluationError::Unknown {
                    message,
                    code,
                    error,
                } => {
                    assert_eq!(message, "Unknown error");
                    assert_eq!(code, 9999);
                    assert_eq!(error, json!({"foo": "bar", "baz": 123}));
                }
                _ => panic!("Expected Unknown variant"),
            }
        }

        #[test]
        fn deserialize_unknown_code_without_data() {
            let json = json!({
                "code": 9999,
                "message": "Unknown error"
            });

            let error: EvaluationError = serde_json::from_value(json).unwrap();

            match error {
                EvaluationError::Unknown { error, .. } => {
                    assert_eq!(error, Value::Null);
                }
                _ => panic!("Expected Unknown variant"),
            }
        }
    }

    mod error_cases {
        use super::*;

        #[test]
        fn missing_required_field_in_data() {
            let json = json!({
                "code": 2,
                "message": "Node too old",
                "data": {
                    "minimumRequiredEra": "Babbage"
                    // missing currentNodeEra
                }
            });

            let result: Result<EvaluationError, _> = serde_json::from_value(json);
            assert!(result.is_err());
        }

        #[test]
        fn missing_data_for_struct_variant() {
            let json = json!({
                "code": 1,
                "message": "Era mismatch"
                // missing data field
            });

            let result: Result<EvaluationError, _> = serde_json::from_value(json);
            assert!(result.is_err());
        }

        #[test]
        fn missing_data_for_single_value_variant() {
            let json = json!({
                "code": 4,
                "message": "Custom error"
                // missing data field
            });

            let result: Result<EvaluationError, _> = serde_json::from_value(json);
            assert!(result.is_err());
        }

        #[test]
        fn wrong_type_in_data() {
            let json = json!({
                "code": 1,
                "message": "Era mismatch",
                "data": {
                    "incompatibleEra": 123  // should be string
                }
            });

            let result: Result<EvaluationError, _> = serde_json::from_value(json);
            assert!(result.is_err());
        }

        #[test]
        fn missing_code_field() {
            let json = json!({
                "message": "Some error",
                "data": {}
            });

            let result: Result<EvaluationError, _> = serde_json::from_value(json);
            assert!(result.is_err());
        }

        #[test]
        fn missing_message_field() {
            let json = json!({
                "code": 1,
                "data": {
                    "incompatibleEra": "Byron"
                }
            });

            let result: Result<EvaluationError, _> = serde_json::from_value(json);
            assert!(result.is_err());
        }
    }

    mod from_str {
        use super::*;

        #[test]
        fn deserialize_from_json_string() {
            let json_str = r#"{
                "code": 1,
                "message": "Context error",
                "data": {
                    "incompatibleEra": "Byron"
                }
            }"#;

            let error: EvaluationError = serde_json::from_str(json_str).unwrap();

            assert_eq!(error.code(), 1);
            assert_eq!(error.message(), "Context error");
            assert_eq!(
                error,
                EvaluationError::IncompatibleEra {
                    message: "Context error".to_string(),
                    incompatible_era: "Byron".to_string(),
                }
            );
        }
    }
}
