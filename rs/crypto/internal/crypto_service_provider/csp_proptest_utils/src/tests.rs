use super::*;
use crate::common::MAX_ALGORITHM_ID_INDEX;

#[test]
fn should_be_maximal_algorithm_index_id_to_ensure_all_variants_covered_by_strategy() {
    assert_eq!(
        AlgorithmId::MegaSecp256k1,
        AlgorithmId::from(MAX_ALGORITHM_ID_INDEX)
    );
    assert_eq!(
        AlgorithmId::Placeholder,
        AlgorithmId::from(MAX_ALGORITHM_ID_INDEX + 1)
    );
}

macro_rules! should_have_a_strategy_for_each_variant {
    ($enum_name:ty, $base_case:expr, $($variant:ident $pattern:tt),+ $(,)?) => {
        paste::paste! {
            #[test]
            fn [<should_have_a_strategy_for_each_variant_of_ $enum_name:snake >]() {
                let error_to_match = $base_case;
                let _ = match error_to_match {
                    $(
                    $enum_name::$variant $pattern => proptest::strategy::Strategy::boxed(
                        crate::[< $enum_name:snake >]::[<arb_ $variant:snake _variant>]()
                    ),
                    )+
                };
            }
        }
    };
}

use ic_crypto_internal_csp::vault::api::CspBasicSignatureError;
should_have_a_strategy_for_each_variant!(
    CspBasicSignatureError,
    CspBasicSignatureError::InternalError {
        internal_error: "dummy error to match upon".to_string(),
    },
    SecretKeyNotFound { .. },
    UnsupportedAlgorithm { .. },
    WrongSecretKeyType { .. },
    MalformedSecretKey { .. },
    InternalError { .. }
);

use ic_crypto_internal_csp::types::CspSignature;
should_have_a_strategy_for_each_variant!(
    CspSignature,
    CspSignature::RsaSha256(b"dummy signature to match upon".to_vec()),
    EcdsaP256(_),
    EcdsaSecp256k1(_),
    Ed25519(_),
    MultiBls12_381(_),
    ThresBls12_381(_),
    RsaSha256(_)
);

use ic_crypto_internal_csp::types::MultiBls12_381_Signature;
use ic_crypto_internal_multi_sig_bls12381::types as multi_types;
should_have_a_strategy_for_each_variant!(
    MultiBls12_381_Signature,
    MultiBls12_381_Signature::Individual(multi_types::IndividualSignatureBytes([42; 48])),
    Individual(_),
    Combined(_)
);

use ic_crypto_internal_csp::types::ThresBls12_381_Signature;
use ic_crypto_internal_threshold_sig_bls12381::types as threshold_types;
should_have_a_strategy_for_each_variant!(
    ThresBls12_381_Signature,
    ThresBls12_381_Signature::Individual(threshold_types::IndividualSignatureBytes([42; 48])),
    Individual(_),
    Combined(_)
);

use ic_crypto_internal_basic_sig_ecdsa_secp256r1::types as ecdsa_secp256r1_types;
use ic_crypto_internal_csp::types::CspPublicKey;
should_have_a_strategy_for_each_variant!(
    CspPublicKey,
    CspPublicKey::EcdsaP256(ecdsa_secp256r1_types::PublicKeyBytes(
        b"dummy value to match upon".to_vec(),
    )),
    EcdsaP256(_),
    EcdsaSecp256k1(_),
    Ed25519(_),
    MultiBls12_381(_),
    RsaSha256(_)
);

use ic_crypto_internal_csp::types::CspPop;
should_have_a_strategy_for_each_variant!(
    CspPop,
    CspPop::MultiBls12_381(multi_types::PopBytes([0; 48])),
    MultiBls12_381(_)
);

use ic_crypto_internal_csp::vault::api::CspBasicSignatureKeygenError;
should_have_a_strategy_for_each_variant!(
    CspBasicSignatureKeygenError,
    CspBasicSignatureKeygenError::InternalError {
        internal_error: "dummy error to match upon".to_string(),
    },
    InternalError { .. },
    DuplicateKeyId { .. },
    TransientInternalError { .. }
);

use ic_crypto_internal_csp::vault::api::CspMultiSignatureError;
should_have_a_strategy_for_each_variant!(
    CspMultiSignatureError,
    CspMultiSignatureError::InternalError {
        internal_error: "dummy error to match upon".to_string(),
    },
    SecretKeyNotFound { .. },
    UnsupportedAlgorithm { .. },
    WrongSecretKeyType { .. },
    InternalError { .. }
);

use ic_crypto_internal_csp::vault::api::CspMultiSignatureKeygenError;
should_have_a_strategy_for_each_variant!(
    CspMultiSignatureKeygenError,
    CspMultiSignatureKeygenError::InternalError {
        internal_error: "dummy error to match upon".to_string(),
    },
    MalformedPublicKey { .. },
    InternalError { .. },
    DuplicateKeyId { .. },
    TransientInternalError { .. }
);

use ic_crypto_internal_csp::api::CspThresholdSignError;
should_have_a_strategy_for_each_variant!(
    CspThresholdSignError,
    CspThresholdSignError::InternalError {
        internal_error: "dummy error to match upon".to_string(),
    },
    SecretKeyNotFound { .. },
    UnsupportedAlgorithm { .. },
    WrongSecretKeyType { .. },
    MalformedSecretKey { .. },
    InternalError { .. }
);
