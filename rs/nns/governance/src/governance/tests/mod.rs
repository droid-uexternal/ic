use super::*;
use crate::{
    pb::v1::{
        governance::{followers_map::Followers, FollowersMap},
        settle_community_fund_participation, Neuron, OpenSnsTokenSwap,
        SettleCommunityFundParticipation,
    },
    test_utils::{
        ExpectedCallCanisterMethodCallArguments, MockEnvironment, StubCMC, StubIcpLedger,
        EXPECTED_LIST_DEPLOYED_SNSES, OPEN_SNS_TOKEN_SWAP, PRINCIPAL_ID_1, PRINCIPAL_ID_2,
        TARGET_SWAP_CANISTER_ID, TEST_SWAP_PARAMS,
    },
};
use candid::Encode;
use ic_base_types::PrincipalId;
use ic_nervous_system_common::{assert_is_err, assert_is_ok, E8};
#[cfg(feature = "test")]
use ic_nervous_system_proto::pb::v1::GlobalTimeOfDay;
use ic_nns_common::pb::v1::NeuronId;
use ic_nns_constants::SNS_WASM_CANISTER_ID;
#[cfg(feature = "test")]
use ic_sns_init::pb::v1::SnsInitPayload;
#[cfg(feature = "test")]
use ic_sns_init::pb::v1::{self as sns_init_pb};
use ic_sns_swap::pb::{v1 as sns_swap_pb, v1::NeuronBasketConstructionParameters};
use ic_sns_wasm::pb::v1::{ListDeployedSnsesRequest, ListDeployedSnsesResponse};
use lazy_static::lazy_static;
use maplit::{btreemap, hashmap};
use std::{convert::TryFrom, string::ToString};

mod stake_maturity;

#[test]
fn test_time_warp() {
    let w = TimeWarp { delta_s: 0_i64 };
    assert_eq!(w.apply(100_u64), 100);

    let w = TimeWarp { delta_s: 42_i64 };
    assert_eq!(w.apply(100_u64), 142);

    let w = TimeWarp { delta_s: -42_i64 };
    assert_eq!(w.apply(100_u64), 58);
}

#[tokio::test]
async fn validate_open_sns_token_swap_ok() {
    let result =
        validate_open_sns_token_swap(&OPEN_SNS_TOKEN_SWAP, &mut MockEnvironment::default()).await;
    assert!(result.is_ok(), "{:#?}", result);
}

#[tokio::test]
async fn validate_open_sns_token_swap_missing_target_swap_canister_id() {
    assert_is_err!(
        validate_open_sns_token_swap(
            &OpenSnsTokenSwap {
                target_swap_canister_id: None,
                ..OPEN_SNS_TOKEN_SWAP.clone()
            },
            &mut MockEnvironment::default(),
        )
        .await
    );
}

#[tokio::test]
async fn validate_open_sns_token_swap_params_no_params() {
    assert_is_err!(
        validate_open_sns_token_swap(
            &OpenSnsTokenSwap {
                params: None,
                ..OPEN_SNS_TOKEN_SWAP.clone()
            },
            &mut MockEnvironment::default(),
        )
        .await
    );
}

#[tokio::test]
async fn validate_open_sns_token_swap_sns_wasm_list_deployed_snses_fail() {
    assert_is_err!(
        validate_open_sns_token_swap(
            &OPEN_SNS_TOKEN_SWAP,
            &mut MockEnvironment::new(
                vec![(
                    ExpectedCallCanisterMethodCallArguments::new(
                        SNS_WASM_CANISTER_ID,
                        "list_deployed_snses",
                        Encode!(&ListDeployedSnsesRequest {}).unwrap(),
                    ),
                    Err((None, "derp".to_string())),
                )],
                0
            ),
        )
        .await
    );
}

#[tokio::test]
async fn validate_open_sns_token_swap_unknown_swap() {
    assert_is_err!(
        validate_open_sns_token_swap(
            &OPEN_SNS_TOKEN_SWAP,
            &mut MockEnvironment::new(
                vec![(
                    ExpectedCallCanisterMethodCallArguments::new(
                        SNS_WASM_CANISTER_ID,
                        "list_deployed_snses",
                        Encode!(&ListDeployedSnsesRequest {}).unwrap(),
                    ),
                    Ok(Encode!(&ListDeployedSnsesResponse { instances: vec![] }).unwrap()),
                )],
                0
            ),
        )
        .await
    );
}

#[tokio::test]
async fn validate_open_sns_token_swap_swap_get_state_fail() {
    assert_is_err!(
        validate_open_sns_token_swap(
            &OPEN_SNS_TOKEN_SWAP,
            &mut MockEnvironment::new(
                vec![
                    EXPECTED_LIST_DEPLOYED_SNSES.clone(),
                    (
                        ExpectedCallCanisterMethodCallArguments::new(
                            *TARGET_SWAP_CANISTER_ID,
                            "get_state",
                            Encode!(&sns_swap_pb::GetStateRequest {}).unwrap(),
                        ),
                        Err((None, "derp".to_string())),
                    ),
                ],
                0
            ),
        )
        .await
    );
}

#[tokio::test]
async fn validate_open_sns_token_swap_params_max_icp_e8s_too_small() {
    assert_is_err!(
        validate_open_sns_token_swap(
            &OpenSnsTokenSwap {
                params: Some(sns_swap_pb::Params {
                    max_icp_e8s: 1, // Too small.
                    ..TEST_SWAP_PARAMS.clone()
                }),
                ..OPEN_SNS_TOKEN_SWAP.clone()
            },
            &mut MockEnvironment::default(),
        )
        .await
    );
}

#[tokio::test]
async fn validate_open_sns_token_swap_params_basket_count_too_small() {
    assert_is_err!(
        validate_open_sns_token_swap(
            &OpenSnsTokenSwap {
                params: Some(sns_swap_pb::Params {
                    neuron_basket_construction_parameters: Some(
                        NeuronBasketConstructionParameters {
                            count: 0,                                 // Too small
                            dissolve_delay_interval_seconds: 7890000, // 3 months
                        },
                    ),
                    ..TEST_SWAP_PARAMS.clone()
                }),
                ..OPEN_SNS_TOKEN_SWAP.clone()
            },
            &mut MockEnvironment::default(),
        )
        .await
    );
}

#[tokio::test]
async fn validate_open_sns_token_swap_params_zero_dissolve_delay() {
    assert_is_err!(
        validate_open_sns_token_swap(
            &OpenSnsTokenSwap {
                params: Some(sns_swap_pb::Params {
                    neuron_basket_construction_parameters: Some(
                        NeuronBasketConstructionParameters {
                            count: 12,
                            dissolve_delay_interval_seconds: 0, // Too small
                        },
                    ),
                    ..TEST_SWAP_PARAMS.clone()
                }),
                ..OPEN_SNS_TOKEN_SWAP.clone()
            },
            &mut MockEnvironment::default(),
        )
        .await
    );
}

#[tokio::test]
async fn validate_open_sns_token_swap_params_practically_forever_dissolve_delay() {
    assert_is_err!(
        validate_open_sns_token_swap(
            &OpenSnsTokenSwap {
                params: Some(sns_swap_pb::Params {
                    neuron_basket_construction_parameters: Some(
                        NeuronBasketConstructionParameters {
                            count: 2,
                            dissolve_delay_interval_seconds: u64::MAX, // Will result in overflow
                        },
                    ),
                    ..TEST_SWAP_PARAMS.clone()
                }),
                ..OPEN_SNS_TOKEN_SWAP.clone()
            },
            &mut MockEnvironment::default(),
        )
        .await
    );
}

#[tokio::test]
async fn validate_open_sns_token_swap_community_fund_investment_e8s() {
    assert_is_err!(
        validate_open_sns_token_swap(
            &OpenSnsTokenSwap {
                community_fund_investment_e8s: Some(1001 * E8), // Exceeds max_icp_e8s.
                ..OPEN_SNS_TOKEN_SWAP.clone()
            },
            &mut MockEnvironment::default(),
        )
        .await
    );
}

lazy_static! {
    static ref NEURON_STORE: NeuronStore = craft_neuron_store(&[
        // (maturity, controller, joined cf at)

        // CF neurons.
        (100 * E8, *PRINCIPAL_ID_1, Some(1)),
        (200 * E8, *PRINCIPAL_ID_2, Some(1)),
        (300 * E8, *PRINCIPAL_ID_1, Some(1)),

        // non-CF neurons.
        (400 * E8, *PRINCIPAL_ID_1, None),
        (500 * E8, *PRINCIPAL_ID_2, None),
    ]);

    static ref ORIGINAL_TOTAL_COMMUNITY_FUND_MATURITY_E8S_EQUIVALENT: u64 = {
        let result = total_community_fund_maturity_e8s_equivalent(&NEURON_STORE);
        assert_eq!(result, 600 * E8);
        result
    };
}

fn craft_neuron_store(
    values: &[(
        /* maturity: */ u64,
        /* controller: */ PrincipalId,
        /* joined cf at: */ Option<u64>,
    )],
) -> NeuronStore {
    NeuronStore::new(
        values
            .iter()
            .enumerate()
            .map(|(i, arg)| {
                let i = i as u64;
                let (maturity_e8s_equivalent, controller, joined_community_fund_timestamp_seconds) =
                    *arg;

                let id = i + 1;
                let neuron = Neuron {
                    id: Some(NeuronId { id }),
                    controller: Some(controller),
                    maturity_e8s_equivalent,
                    joined_community_fund_timestamp_seconds,
                    ..Default::default()
                };

                (id, neuron)
            })
            .collect(),
    )
}

fn assert_clean_refund(
    neuron_store: &mut NeuronStore,
    cf_participants: &Vec<sns_swap_pb::CfParticipant>,
    expected_neuron_store: &NeuronStore,
) {
    let original_id_to_neuron = neuron_store.clone_neurons();
    let mut original_neuron_store = NeuronStore::new(original_id_to_neuron);

    let failed_refunds = refund_community_fund_maturity(neuron_store, cf_participants);
    assert!(failed_refunds.is_empty(), "{:#?}", failed_refunds);

    // Assert that neurons have been restored to the way they were originally.
    assert_eq!(neuron_store, expected_neuron_store);

    // Assert that inserting extraneous elements into cf_participants does
    // not change the result, but it does result in failed refunds.
    let mut extra_cf_participants = cf_participants.clone();
    let mut expected_failed_refunds = vec![];
    if !extra_cf_participants.is_empty() {
        let cf_neuron = sns_swap_pb::CfNeuron::try_new(688477, 592).unwrap();
        extra_cf_participants
            .get_mut(0)
            .unwrap()
            .cf_neurons
            .push(cf_neuron.clone());
        expected_failed_refunds.push(sns_swap_pb::CfParticipant {
            hotkey_principal: extra_cf_participants
                .first()
                .unwrap()
                .hotkey_principal
                .clone(),
            cf_neurons: vec![cf_neuron],
        });
    }

    let cf_participant = sns_swap_pb::CfParticipant {
        hotkey_principal: PrincipalId::new_user_test_id(301590).to_string(),
        cf_neurons: vec![
            sns_swap_pb::CfNeuron::try_new(875889, 591).unwrap(),
            sns_swap_pb::CfNeuron::try_new(734429, 917).unwrap(),
        ],
    };
    extra_cf_participants.push(cf_participant.clone());
    expected_failed_refunds.push(cf_participant);

    assert_eq!(
        refund_community_fund_maturity(&mut original_neuron_store, &extra_cf_participants,),
        expected_failed_refunds,
    );
    assert_eq!(original_neuron_store, *expected_neuron_store);
}

#[test]
fn draw_funds_from_the_community_fund_all_cf_neurons_have_zero_maturity() {
    let mut neuron_store = craft_neuron_store(&[
        // (maturity, controller, joined cf at)

        // CF neurons.
        (0, *PRINCIPAL_ID_1, Some(1)),
        (0, *PRINCIPAL_ID_2, Some(1)),
        (0, *PRINCIPAL_ID_1, Some(1)),
        // non-CF neurons.
        (400, *PRINCIPAL_ID_1, None),
        (500, *PRINCIPAL_ID_2, None),
    ]);
    let original_neuron_store = neuron_store.clone();

    let observed_cf_neurons = draw_funds_from_the_community_fund(
        &mut neuron_store,
        *ORIGINAL_TOTAL_COMMUNITY_FUND_MATURITY_E8S_EQUIVALENT,
        /* withdrawal_amount_e8s = */ 60,
        &TEST_SWAP_PARAMS,
    );

    // Inspect results.
    assert_eq!(observed_cf_neurons, vec![]);
    assert_eq!(neuron_store, original_neuron_store);
    assert_clean_refund(
        &mut neuron_store,
        &observed_cf_neurons,
        &original_neuron_store,
    );
}

#[test]
fn draw_funds_from_the_community_fund_zero_withdrawal_amount() {
    let mut neuron_store = craft_neuron_store(&[
        // (maturity, controller, joined cf at)

        // CF neurons.
        (0, *PRINCIPAL_ID_1, Some(1)),
        (10, *PRINCIPAL_ID_2, Some(1)),
        (50, *PRINCIPAL_ID_1, Some(1)),
        // non-CF neurons.
        (400, *PRINCIPAL_ID_1, None),
        (500, *PRINCIPAL_ID_2, None),
    ]);
    let original_neuron_store = neuron_store.clone();

    let observed_cf_neurons = draw_funds_from_the_community_fund(
        &mut neuron_store,
        *ORIGINAL_TOTAL_COMMUNITY_FUND_MATURITY_E8S_EQUIVALENT,
        /* withdrawal_amount_e8s = */ 0,
        &TEST_SWAP_PARAMS,
    );

    // Inspect results.
    assert_eq!(observed_cf_neurons, vec![]);
    assert_eq!(neuron_store, original_neuron_store);
    assert_clean_refund(
        &mut neuron_store,
        &observed_cf_neurons,
        &original_neuron_store,
    );
}

#[test]
fn draw_funds_from_the_community_fund_typical() {
    let mut neuron_store = NEURON_STORE.clone();
    let observed_cf_neurons = draw_funds_from_the_community_fund(
        &mut neuron_store,
        *ORIGINAL_TOTAL_COMMUNITY_FUND_MATURITY_E8S_EQUIVALENT,
        /* withdrawal_amount_e8s = */ 60 * E8,
        &TEST_SWAP_PARAMS,
    );

    // Inspect results.

    let mut expected_cf_neurons = vec![
        sns_swap_pb::CfParticipant {
            hotkey_principal: PRINCIPAL_ID_1.to_string(),
            cf_neurons: vec![
                sns_swap_pb::CfNeuron::try_new(1, 10 * E8).unwrap(),
                sns_swap_pb::CfNeuron::try_new(3, 30 * E8).unwrap(),
            ],
        },
        sns_swap_pb::CfParticipant {
            hotkey_principal: PRINCIPAL_ID_2.to_string(),
            cf_neurons: vec![sns_swap_pb::CfNeuron::try_new(2, 20 * E8).unwrap()],
        },
    ];
    expected_cf_neurons.sort_by(|n1, n2| n1.hotkey_principal.cmp(&n2.hotkey_principal));
    assert_eq!(observed_cf_neurons, expected_cf_neurons);

    assert_eq!(
        neuron_store,
        craft_neuron_store(&[
            // CF neurons less 10% of their maturity.
            (90 * E8, *PRINCIPAL_ID_1, Some(1)),
            (180 * E8, *PRINCIPAL_ID_2, Some(1)),
            (270 * E8, *PRINCIPAL_ID_1, Some(1)),
            // non-CF neurons remain untouched.
            (400 * E8, *PRINCIPAL_ID_1, None),
            (500 * E8, *PRINCIPAL_ID_2, None),
        ]),
    );

    assert_clean_refund(&mut neuron_store, &observed_cf_neurons, &NEURON_STORE);
}

#[test]
fn draw_funds_from_the_community_fund_cf_shrank_during_voting_period() {
    let mut neuron_store = NEURON_STORE.clone();

    let observed_cf_neurons = draw_funds_from_the_community_fund(
        &mut neuron_store,
        2 * *ORIGINAL_TOTAL_COMMUNITY_FUND_MATURITY_E8S_EQUIVALENT,
        /* withdrawal_amount_e8s = */ 60 * E8,
        &TEST_SWAP_PARAMS,
    );

    // Inspect results.

    let mut expected_cf_neurons = vec![
        sns_swap_pb::CfParticipant {
            hotkey_principal: PRINCIPAL_ID_1.to_string(),
            cf_neurons: vec![
                sns_swap_pb::CfNeuron::try_new(1, 5 * E8).unwrap(),
                sns_swap_pb::CfNeuron::try_new(3, 15 * E8).unwrap(),
            ],
        },
        sns_swap_pb::CfParticipant {
            hotkey_principal: PRINCIPAL_ID_2.to_string(),
            cf_neurons: vec![sns_swap_pb::CfNeuron::try_new(2, 10 * E8).unwrap()],
        },
    ];
    expected_cf_neurons.sort_by(|n1, n2| n1.hotkey_principal.cmp(&n2.hotkey_principal));
    assert_eq!(observed_cf_neurons, expected_cf_neurons);

    assert_eq!(
        neuron_store,
        craft_neuron_store(&[
            // CF neurons less 10% of their maturity.
            (95 * E8, *PRINCIPAL_ID_1, Some(1)),
            (190 * E8, *PRINCIPAL_ID_2, Some(1)),
            (285 * E8, *PRINCIPAL_ID_1, Some(1)),
            // non-CF neurons remain untouched.
            (400 * E8, *PRINCIPAL_ID_1, None),
            (500 * E8, *PRINCIPAL_ID_2, None),
        ]),
    );

    assert_clean_refund(&mut neuron_store, &observed_cf_neurons, &NEURON_STORE);
}

#[test]
fn draw_funds_from_the_community_fund_cf_grew_during_voting_period() {
    let mut neuron_store = NEURON_STORE.clone();

    let observed_cf_neurons = draw_funds_from_the_community_fund(
        &mut neuron_store,
        *ORIGINAL_TOTAL_COMMUNITY_FUND_MATURITY_E8S_EQUIVALENT / 2,
        /* withdrawal_amount_e8s = */ 60 * E8,
        &TEST_SWAP_PARAMS,
    );

    // Inspect results. Same as typical (copy n' pasted).

    let mut expected_cf_neurons = vec![
        sns_swap_pb::CfParticipant {
            hotkey_principal: PRINCIPAL_ID_1.to_string(),
            cf_neurons: vec![
                sns_swap_pb::CfNeuron::try_new(1, 10 * E8).unwrap(),
                sns_swap_pb::CfNeuron::try_new(3, 30 * E8).unwrap(),
            ],
        },
        sns_swap_pb::CfParticipant {
            hotkey_principal: PRINCIPAL_ID_2.to_string(),
            cf_neurons: vec![sns_swap_pb::CfNeuron::try_new(2, 20 * E8).unwrap()],
        },
    ];
    expected_cf_neurons.sort_by(|n1, n2| n1.hotkey_principal.cmp(&n2.hotkey_principal));
    assert_eq!(observed_cf_neurons, expected_cf_neurons);

    assert_eq!(
        neuron_store,
        craft_neuron_store(&[
            // CF neurons less 10% of their maturity.
            (90 * E8, *PRINCIPAL_ID_1, Some(1)),
            (180 * E8, *PRINCIPAL_ID_2, Some(1)),
            (270 * E8, *PRINCIPAL_ID_1, Some(1)),
            // non-CF neurons remain untouched.
            (400 * E8, *PRINCIPAL_ID_1, None),
            (500 * E8, *PRINCIPAL_ID_2, None),
        ]),
    );

    assert_clean_refund(&mut neuron_store, &observed_cf_neurons, &NEURON_STORE);
}

#[test]
fn draw_funds_from_the_community_fund_trivial() {
    let mut neuron_store = NeuronStore::new(btreemap! {});
    let original_total_community_fund_maturity_e8s_equivalent = 0;

    let observed_cf_neurons = draw_funds_from_the_community_fund(
        &mut neuron_store,
        original_total_community_fund_maturity_e8s_equivalent,
        /* withdrawal_amount_e8s = */ 60,
        &TEST_SWAP_PARAMS,
    );

    // Inspect results.
    assert_eq!(observed_cf_neurons, vec![]);
    assert_eq!(neuron_store, NeuronStore::new(btreemap! {}));

    assert_clean_refund(
        &mut neuron_store,
        &observed_cf_neurons,
        &NeuronStore::new(btreemap! {}),
    );
}

#[test]
fn draw_funds_from_the_community_fund_cf_not_large_enough() {
    let mut neuron_store = NEURON_STORE.clone();

    let observed_cf_neurons = draw_funds_from_the_community_fund(
        &mut neuron_store,
        *ORIGINAL_TOTAL_COMMUNITY_FUND_MATURITY_E8S_EQUIVALENT,
        /* withdrawal_amount_e8s = */ 1000 * E8,
        &TEST_SWAP_PARAMS,
    );

    // Inspect results.

    let mut expected_cf_neurons = vec![
        sns_swap_pb::CfParticipant {
            hotkey_principal: PRINCIPAL_ID_1.to_string(),
            cf_neurons: vec![
                sns_swap_pb::CfNeuron::try_new(1, 100 * E8).unwrap(),
                sns_swap_pb::CfNeuron::try_new(3, 300 * E8).unwrap(),
            ],
        },
        sns_swap_pb::CfParticipant {
            hotkey_principal: PRINCIPAL_ID_2.to_string(),
            cf_neurons: vec![sns_swap_pb::CfNeuron::try_new(2, 200 * E8).unwrap()],
        },
    ];
    expected_cf_neurons.sort_by(|n1, n2| n1.hotkey_principal.cmp(&n2.hotkey_principal));
    assert_eq!(observed_cf_neurons, expected_cf_neurons);

    assert_eq!(
        neuron_store,
        craft_neuron_store(&[
            // CF neurons have been completely depleted.
            (0, *PRINCIPAL_ID_1, Some(1)),
            (0, *PRINCIPAL_ID_2, Some(1)),
            (0, *PRINCIPAL_ID_1, Some(1)),
            // non-CF neurons remain untouched.
            (400 * E8, *PRINCIPAL_ID_1, None),
            (500 * E8, *PRINCIPAL_ID_2, None),
        ]),
    );

    assert_clean_refund(&mut neuron_store, &observed_cf_neurons, &NEURON_STORE);
}

#[test]
fn draw_funds_from_the_community_fund_exclude_small_cf_neuron_and_cap_large() {
    let params = Params {
        min_participant_icp_e8s: 150 * E8,
        max_participant_icp_e8s: 225 * E8,
        ..TEST_SWAP_PARAMS.clone()
    };
    let mut neuron_store = NEURON_STORE.clone();

    let observed_cf_neurons = draw_funds_from_the_community_fund(
        &mut neuron_store,
        *ORIGINAL_TOTAL_COMMUNITY_FUND_MATURITY_E8S_EQUIVALENT,
        /* withdrawal_amount_e8s = */ 600 * E8,
        &params,
    );

    // Inspect results.

    let mut expected_cf_neurons = vec![
        sns_swap_pb::CfParticipant {
            hotkey_principal: PRINCIPAL_ID_1.to_string(),
            cf_neurons: vec![sns_swap_pb::CfNeuron::try_new(3, 225 * E8).unwrap()],
        },
        sns_swap_pb::CfParticipant {
            hotkey_principal: PRINCIPAL_ID_2.to_string(),
            cf_neurons: vec![sns_swap_pb::CfNeuron::try_new(2, 200 * E8).unwrap()],
        },
    ];
    expected_cf_neurons.sort_by(|n1, n2| n1.hotkey_principal.cmp(&n2.hotkey_principal));
    assert_eq!(observed_cf_neurons, expected_cf_neurons);

    assert_eq!(
        neuron_store,
        craft_neuron_store(&[
            // CF neurons.
            (100 * E8, *PRINCIPAL_ID_1, Some(1)), // Does not participate, because too small.
            (0, *PRINCIPAL_ID_2, Some(1)),        // Fully participates.
            (75 * E8, *PRINCIPAL_ID_1, Some(1)),  // Participates up to the allowed participant max.
            // non-CF neurons remain untouched.
            (400 * E8, *PRINCIPAL_ID_1, None),
            (500 * E8, *PRINCIPAL_ID_2, None),
        ]),
    );

    assert_clean_refund(&mut neuron_store, &observed_cf_neurons, &NEURON_STORE);
}

#[test]
fn sum_cf_participants_e8s_nonempty() {
    assert_eq!(
        sum_cf_participants_e8s(&[
            sns_swap_pb::CfParticipant {
                hotkey_principal: PRINCIPAL_ID_1.to_string(),
                cf_neurons: vec![
                    sns_swap_pb::CfNeuron::try_new(1, 100,).unwrap(),
                    sns_swap_pb::CfNeuron::try_new(3, 300,).unwrap(),
                ],
            },
            sns_swap_pb::CfParticipant {
                hotkey_principal: PRINCIPAL_ID_2.to_string(),
                cf_neurons: vec![sns_swap_pb::CfNeuron::try_new(2, 200,).unwrap()],
            },
        ]),
        600,
    );
}

// TODO[NNS1-2632]: Remove this test once `settle_community_fund_participation` is deprecated.
mod settle_community_fund_participation_tests {
    use settle_community_fund_participation::{Aborted, Committed, Result};

    use super::*;

    lazy_static! {
        static ref COMMITTED: SettleCommunityFundParticipation = SettleCommunityFundParticipation {
            open_sns_token_swap_proposal_id: Some(7),
            result: Some(Result::Committed(Committed {
                sns_governance_canister_id: Some(PrincipalId::new_user_test_id(672891)),
                total_direct_contribution_icp_e8s: None,
                total_neurons_fund_contribution_icp_e8s: None,
            })),
        };
        static ref ABORTED: SettleCommunityFundParticipation = SettleCommunityFundParticipation {
            open_sns_token_swap_proposal_id: Some(42),
            result: Some(Result::Aborted(Aborted {})),
        };
    }

    #[test]
    fn ok() {
        assert_is_ok!(validate_settle_community_fund_participation(&COMMITTED));
        assert_is_ok!(validate_settle_community_fund_participation(&ABORTED));
    }

    #[test]
    fn no_proposal_id() {
        assert_is_err!(validate_settle_community_fund_participation(
            &SettleCommunityFundParticipation {
                open_sns_token_swap_proposal_id: None,
                ..COMMITTED.clone()
            }
        ));
    }

    #[test]
    fn no_result() {
        assert_is_err!(validate_settle_community_fund_participation(
            &SettleCommunityFundParticipation {
                result: None,
                ..COMMITTED.clone()
            }
        ));
    }

    #[test]
    fn no_sns_governance_canister_id() {
        assert_is_err!(validate_settle_community_fund_participation(
            &SettleCommunityFundParticipation {
                open_sns_token_swap_proposal_id: Some(7),
                result: Some(Result::Committed(Committed {
                    sns_governance_canister_id: None,
                    total_direct_contribution_icp_e8s: None,
                    total_neurons_fund_contribution_icp_e8s: None,
                })),
            }
        ));
    }
} // end mod settle_community_fund_participation_tests

mod settle_neurons_fund_participation_request_tests {
    use settle_neurons_fund_participation_request::{Aborted, Committed, Result};
    use SettleNeuronsFundParticipationRequest;

    use super::*;

    lazy_static! {
        static ref COMMITTED: SettleNeuronsFundParticipationRequest =
            SettleNeuronsFundParticipationRequest {
                nns_proposal_id: Some(7),
                result: Some(Result::Committed(Committed {
                    sns_governance_canister_id: Some(PrincipalId::new_user_test_id(672891)),
                    total_direct_participation_icp_e8s: Some(100_000 * E8),
                    total_neurons_fund_participation_icp_e8s: Some(50_000 * E8),
                }))
            };
        static ref ABORTED: SettleNeuronsFundParticipationRequest =
            SettleNeuronsFundParticipationRequest {
                nns_proposal_id: Some(42),
                result: Some(Result::Aborted(Aborted {}))
            };
    }

    #[test]
    fn ok() {
        assert_is_ok!(ValidatedSettleNeuronsFundParticipationRequest::try_from(
            COMMITTED.clone()
        ));
        assert_is_ok!(ValidatedSettleNeuronsFundParticipationRequest::try_from(
            ABORTED.clone()
        ));
    }

    #[test]
    fn no_proposal_id() {
        assert_is_err!(ValidatedSettleNeuronsFundParticipationRequest::try_from(
            SettleNeuronsFundParticipationRequest {
                nns_proposal_id: None,
                ..COMMITTED.clone()
            }
        ));
    }

    #[test]
    fn no_result() {
        assert_is_err!(ValidatedSettleNeuronsFundParticipationRequest::try_from(
            SettleNeuronsFundParticipationRequest {
                result: None,
                ..COMMITTED.clone()
            }
        ));
    }

    #[test]
    fn no_sns_governance_canister_id() {
        assert_is_err!(ValidatedSettleNeuronsFundParticipationRequest::try_from(
            SettleNeuronsFundParticipationRequest {
                nns_proposal_id: Some(7),
                result: Some(Result::Committed(Committed {
                    sns_governance_canister_id: None,
                    total_direct_participation_icp_e8s: Some(100_000 * E8),
                    total_neurons_fund_participation_icp_e8s: Some(50_000 * E8),
                })),
            }
        ));
    }

    #[test]
    fn no_total_direct_participation_icp_e8s() {
        assert_is_err!(ValidatedSettleNeuronsFundParticipationRequest::try_from(
            SettleNeuronsFundParticipationRequest {
                nns_proposal_id: Some(7),
                result: Some(Result::Committed(Committed {
                    sns_governance_canister_id: Some(PrincipalId::new_user_test_id(672891)),
                    total_direct_participation_icp_e8s: None,
                    total_neurons_fund_participation_icp_e8s: Some(50_000 * E8),
                })),
            }
        ));
    }

    #[test]
    fn no_total_neurons_fund_participation_icp_e8s() {
        assert_is_err!(ValidatedSettleNeuronsFundParticipationRequest::try_from(
            SettleNeuronsFundParticipationRequest {
                nns_proposal_id: Some(7),
                result: Some(Result::Committed(Committed {
                    sns_governance_canister_id: Some(PrincipalId::new_user_test_id(672891)),
                    total_direct_participation_icp_e8s: Some(100_000 * E8),
                    total_neurons_fund_participation_icp_e8s: None,
                })),
            }
        ));
    }
} // end mod settle_neurons_fund_participation_request_tests

#[cfg(feature = "test")]
mod convert_from_create_service_nervous_system_to_sns_init_payload_tests {
    use super::*;
    use ic_nervous_system_proto::pb::v1 as pb;
    use ic_sns_init::pb::v1::sns_init_payload;
    use test_data::{CREATE_SERVICE_NERVOUS_SYSTEM_WITH_MATCHED_FUNDING, IMAGE_1, IMAGE_2};

    // Alias types from crate::pb::v1::...
    //
    // This is done within another mod to differentiate against types that have
    // similar names as types found in ic_sns_init.
    mod src {
        pub use crate::pb::v1::create_service_nervous_system::initial_token_distribution::SwapDistribution;
    }

    #[track_caller]
    fn unwrap_duration_seconds(original: &Option<pb::Duration>) -> Option<u64> {
        Some(original.as_ref().unwrap().seconds.unwrap())
    }

    #[track_caller]
    fn unwrap_tokens_e8s(original: &Option<pb::Tokens>) -> Option<u64> {
        Some(original.as_ref().unwrap().e8s.unwrap())
    }

    #[track_caller]
    fn unwrap_percentage_basis_points(original: &Option<pb::Percentage>) -> Option<u64> {
        Some(original.as_ref().unwrap().basis_points.unwrap())
    }

    #[cfg(feature = "test")]
    #[test]
    fn test_convert_from_valid() {
        // Step 1: Prepare the world. (In this case, trivial.)

        // Step 2: Call the code under test.
        let converted =
            SnsInitPayload::try_from(CREATE_SERVICE_NERVOUS_SYSTEM_WITH_MATCHED_FUNDING.clone())
                .unwrap();

        // Step 3: Inspect the result.

        let original_ledger_parameters: &_ = CREATE_SERVICE_NERVOUS_SYSTEM_WITH_MATCHED_FUNDING
            .ledger_parameters
            .as_ref()
            .unwrap();
        let original_governance_parameters: &_ = CREATE_SERVICE_NERVOUS_SYSTEM_WITH_MATCHED_FUNDING
            .governance_parameters
            .as_ref()
            .unwrap();

        let original_voting_reward_parameters: &_ = original_governance_parameters
            .voting_reward_parameters
            .as_ref()
            .unwrap();

        let original_swap_parameters: &_ = CREATE_SERVICE_NERVOUS_SYSTEM_WITH_MATCHED_FUNDING
            .swap_parameters
            .as_ref()
            .unwrap();

        assert_eq!(
            SnsInitPayload {
                // We'll look at this separately.
                initial_token_distribution: None,
                swap_start_timestamp_seconds: None,
                swap_due_timestamp_seconds: None,
                neurons_fund_participants: None,
                nns_proposal_id: None,
                neuron_basket_construction_parameters: None,
                ..converted
            },
            SnsInitPayload {
                transaction_fee_e8s: unwrap_tokens_e8s(&original_ledger_parameters.transaction_fee),
                token_name: Some(original_ledger_parameters.clone().token_name.unwrap()),
                token_symbol: Some(original_ledger_parameters.clone().token_symbol.unwrap()),
                token_logo: Some(IMAGE_2.to_string()),

                proposal_reject_cost_e8s: unwrap_tokens_e8s(
                    &original_governance_parameters.proposal_rejection_fee
                ),

                neuron_minimum_stake_e8s: unwrap_tokens_e8s(
                    &original_governance_parameters.neuron_minimum_stake
                ),

                fallback_controller_principal_ids:
                    CREATE_SERVICE_NERVOUS_SYSTEM_WITH_MATCHED_FUNDING
                        .fallback_controller_principal_ids
                        .iter()
                        .map(|id| id.to_string())
                        .collect(),

                logo: Some(IMAGE_1.to_string(),),
                url: Some("https://best.app".to_string(),),
                name: Some("Hello, world!".to_string(),),
                description: Some("Best app that you ever did saw.".to_string(),),

                neuron_minimum_dissolve_delay_to_vote_seconds: unwrap_duration_seconds(
                    &original_governance_parameters.neuron_minimum_dissolve_delay_to_vote
                ),

                initial_reward_rate_basis_points: unwrap_percentage_basis_points(
                    &original_voting_reward_parameters.initial_reward_rate
                ),
                final_reward_rate_basis_points: unwrap_percentage_basis_points(
                    &original_voting_reward_parameters.final_reward_rate
                ),
                reward_rate_transition_duration_seconds: unwrap_duration_seconds(
                    &original_voting_reward_parameters.reward_rate_transition_duration
                ),

                max_dissolve_delay_seconds: unwrap_duration_seconds(
                    &original_governance_parameters.neuron_maximum_dissolve_delay
                ),

                max_neuron_age_seconds_for_age_bonus: unwrap_duration_seconds(
                    &original_governance_parameters.neuron_maximum_age_for_age_bonus
                ),

                max_dissolve_delay_bonus_percentage: unwrap_percentage_basis_points(
                    &original_governance_parameters.neuron_maximum_dissolve_delay_bonus
                )
                .map(|basis_points| basis_points / 100),

                max_age_bonus_percentage: unwrap_percentage_basis_points(
                    &original_governance_parameters.neuron_maximum_age_bonus
                )
                .map(|basis_points| basis_points / 100),

                initial_voting_period_seconds: unwrap_duration_seconds(
                    &original_governance_parameters.proposal_initial_voting_period
                ),
                wait_for_quiet_deadline_increase_seconds: unwrap_duration_seconds(
                    &original_governance_parameters.proposal_wait_for_quiet_deadline_increase
                ),
                dapp_canisters: Some(sns_init_pb::DappCanisters {
                    canisters: vec![pb::Canister {
                        id: Some(CanisterId::from_u64(1000).get()),
                    }],
                }),
                min_participants: original_swap_parameters.minimum_participants,
                min_icp_e8s: None,
                max_icp_e8s: None,
                min_direct_participation_icp_e8s: unwrap_tokens_e8s(
                    &original_swap_parameters.minimum_direct_participation_icp
                ),
                max_direct_participation_icp_e8s: unwrap_tokens_e8s(
                    &original_swap_parameters.maximum_direct_participation_icp
                ),
                min_participant_icp_e8s: unwrap_tokens_e8s(
                    &original_swap_parameters.minimum_participant_icp
                ),
                max_participant_icp_e8s: unwrap_tokens_e8s(
                    &original_swap_parameters.maximum_participant_icp
                ),

                confirmation_text: original_swap_parameters.confirmation_text.clone(),
                restricted_countries: original_swap_parameters.restricted_countries.clone(),
                neurons_fund_participation: original_swap_parameters.neurons_fund_participation,

                // We'll examine these later
                initial_token_distribution: None,
                neuron_basket_construction_parameters: None,
                neurons_fund_participants: None,
                swap_start_timestamp_seconds: None,
                swap_due_timestamp_seconds: None,
                nns_proposal_id: None,
                neurons_fund_participation_constraints: None,
            },
        );

        let original_initial_token_distribution: &_ =
            CREATE_SERVICE_NERVOUS_SYSTEM_WITH_MATCHED_FUNDING
                .initial_token_distribution
                .as_ref()
                .unwrap();
        let original_developer_distribution: &_ = original_initial_token_distribution
            .developer_distribution
            .as_ref()
            .unwrap();
        assert_eq!(
            original_developer_distribution.developer_neurons.len(),
            1,
            "{:#?}",
            original_developer_distribution.developer_neurons,
        );
        let original_neuron_distribution: &_ = original_developer_distribution
            .developer_neurons
            .first()
            .unwrap();

        let src::SwapDistribution { total: swap_total } = original_initial_token_distribution
            .swap_distribution
            .as_ref()
            .unwrap();
        let swap_total_e8s = unwrap_tokens_e8s(swap_total).unwrap();
        assert_eq!(swap_total_e8s, 1_840_880_000);

        assert_eq!(
            converted.initial_token_distribution.unwrap(),
            sns_init_payload::InitialTokenDistribution::FractionalDeveloperVotingPower(
                sns_init_pb::FractionalDeveloperVotingPower {
                    developer_distribution: Some(sns_init_pb::DeveloperDistribution {
                        developer_neurons: vec![sns_init_pb::NeuronDistribution {
                            controller: Some(original_neuron_distribution.controller.unwrap()),

                            stake_e8s: unwrap_tokens_e8s(&original_neuron_distribution.stake)
                                .unwrap(),

                            memo: original_neuron_distribution.memo.unwrap(),

                            dissolve_delay_seconds: unwrap_duration_seconds(
                                &original_neuron_distribution.dissolve_delay
                            )
                            .unwrap(),

                            vesting_period_seconds: unwrap_duration_seconds(
                                &original_neuron_distribution.vesting_period
                            ),
                        },],
                    },),
                    treasury_distribution: Some(sns_init_pb::TreasuryDistribution {
                        total_e8s: unwrap_tokens_e8s(
                            &original_initial_token_distribution
                                .treasury_distribution
                                .as_ref()
                                .unwrap()
                                .total
                        )
                        .unwrap(),
                    },),
                    swap_distribution: Some(sns_init_pb::SwapDistribution {
                        // These are intentionally the same.
                        total_e8s: swap_total_e8s,
                        initial_swap_amount_e8s: swap_total_e8s,
                    },),
                    airdrop_distribution: Some(sns_init_pb::AirdropDistribution {
                        airdrop_neurons: vec![],
                    },),
                },
            ),
        );

        let original_neuron_basket_construction_parameters =
            CREATE_SERVICE_NERVOUS_SYSTEM_WITH_MATCHED_FUNDING
                .swap_parameters
                .as_ref()
                .unwrap()
                .neuron_basket_construction_parameters
                .as_ref()
                .unwrap();

        assert_eq!(
            converted.neuron_basket_construction_parameters.unwrap(),
            NeuronBasketConstructionParameters {
                count: original_neuron_basket_construction_parameters
                    .count
                    .unwrap(),
                dissolve_delay_interval_seconds: unwrap_duration_seconds(
                    &original_neuron_basket_construction_parameters.dissolve_delay_interval
                )
                .unwrap(),
            }
        );

        assert_eq!(converted.nns_proposal_id, None);
        assert_eq!(converted.neurons_fund_participants, None);
        assert_eq!(converted.swap_start_timestamp_seconds, None);
        assert_eq!(converted.swap_due_timestamp_seconds, None);
    }

    #[cfg(feature = "test")]
    #[test]
    fn test_convert_from_invalid() {
        // Step 1: Prepare the world: construct input.
        let mut original = CREATE_SERVICE_NERVOUS_SYSTEM_WITH_MATCHED_FUNDING.clone();
        let governance_parameters = original.governance_parameters.as_mut().unwrap();

        // Corrupt the data. The problem with this is that wait for quiet extension
        // amount cannot be more than half the initial voting period.
        governance_parameters.proposal_wait_for_quiet_deadline_increase = governance_parameters
            .proposal_initial_voting_period
            .as_ref()
            .map(|duration| {
                let seconds = Some(duration.seconds.unwrap() / 2 + 1);
                pb::Duration { seconds }
            });

        // Step 2: Call the code under test.
        let converted = SnsInitPayload::try_from(original);

        // Step 3: Inspect the result: Err must contain "wait for quiet".
        match converted {
            Ok(ok) => panic!("Invalid data was not rejected. Result: {:#?}", ok),
            Err(err) => assert!(err.contains("wait_for_quiet"), "{}", err),
        }
    }
}

#[cfg(feature = "test")]
mod convert_from_executed_create_service_nervous_system_proposal_to_sns_init_payload_tests_with_test_feature {
    use super::*;
    use ic_nervous_system_proto::pb::v1 as pb;
    use ic_sns_init::pb::v1::sns_init_payload;
    use test_data::{CREATE_SERVICE_NERVOUS_SYSTEM_WITH_MATCHED_FUNDING, IMAGE_1, IMAGE_2};

    // Alias types from crate::pb::v1::...
    //
    // This is done within another mod to differentiate against types that have
    // similar names as types found in ic_sns_init.
    mod src {
        pub use crate::pb::v1::create_service_nervous_system::initial_token_distribution::SwapDistribution;
    }

    #[track_caller]
    fn unwrap_duration_seconds(original: &Option<pb::Duration>) -> Option<u64> {
        Some(original.as_ref().unwrap().seconds.unwrap())
    }

    #[track_caller]
    fn unwrap_tokens_e8s(original: &Option<pb::Tokens>) -> Option<u64> {
        Some(original.as_ref().unwrap().e8s.unwrap())
    }

    #[track_caller]
    fn unwrap_percentage_basis_points(original: &Option<pb::Percentage>) -> Option<u64> {
        Some(original.as_ref().unwrap().basis_points.unwrap())
    }

    #[test]
    fn test_convert_from_valid() {
        // Step 1: Prepare the world. (In this case, trivial.)

        use ic_sns_init::pb::v1::NeuronsFundParticipants;

        use crate::governance::test_data::NEURONS_FUND_PARTICIPATION_CONSTRAINTS;
        let current_timestamp_seconds = 13_245;
        let proposal_id = 1000;

        let executed_create_service_nervous_system_proposal =
            ExecutedCreateServiceNervousSystemProposal {
                current_timestamp_seconds,
                create_service_nervous_system: CREATE_SERVICE_NERVOUS_SYSTEM_WITH_MATCHED_FUNDING
                    .clone(),
                proposal_id,
                neurons_fund_participants: vec![],
                random_swap_start_time: GlobalTimeOfDay {
                    seconds_after_utc_midnight: Some(0),
                },
                neurons_fund_participation_constraints: Some(
                    NEURONS_FUND_PARTICIPATION_CONSTRAINTS.clone(),
                ),
            };

        // Step 2: Call the code under test.
        let converted =
            SnsInitPayload::try_from(executed_create_service_nervous_system_proposal).unwrap();

        // Step 3: Inspect the result.

        let original_ledger_parameters: &_ = CREATE_SERVICE_NERVOUS_SYSTEM_WITH_MATCHED_FUNDING
            .ledger_parameters
            .as_ref()
            .unwrap();
        let original_governance_parameters: &_ = CREATE_SERVICE_NERVOUS_SYSTEM_WITH_MATCHED_FUNDING
            .governance_parameters
            .as_ref()
            .unwrap();

        let original_voting_reward_parameters: &_ = original_governance_parameters
            .voting_reward_parameters
            .as_ref()
            .unwrap();

        let original_swap_parameters: &_ = CREATE_SERVICE_NERVOUS_SYSTEM_WITH_MATCHED_FUNDING
            .swap_parameters
            .as_ref()
            .unwrap();

        assert_eq!(
            SnsInitPayload {
                // We'll look at this separately.
                initial_token_distribution: None,
                neuron_basket_construction_parameters: None,
                swap_start_timestamp_seconds: None,
                swap_due_timestamp_seconds: None,
                ..converted
            },
            SnsInitPayload {
                transaction_fee_e8s: unwrap_tokens_e8s(&original_ledger_parameters.transaction_fee),
                token_name: Some(original_ledger_parameters.clone().token_name.unwrap()),
                token_symbol: Some(original_ledger_parameters.clone().token_symbol.unwrap()),
                token_logo: Some(IMAGE_2.to_string()),

                proposal_reject_cost_e8s: unwrap_tokens_e8s(
                    &original_governance_parameters.proposal_rejection_fee
                ),

                neuron_minimum_stake_e8s: unwrap_tokens_e8s(
                    &original_governance_parameters.neuron_minimum_stake
                ),

                fallback_controller_principal_ids:
                    CREATE_SERVICE_NERVOUS_SYSTEM_WITH_MATCHED_FUNDING
                        .fallback_controller_principal_ids
                        .iter()
                        .map(|id| id.to_string())
                        .collect(),

                logo: Some(IMAGE_1.to_string(),),
                url: Some("https://best.app".to_string(),),
                name: Some("Hello, world!".to_string(),),
                description: Some("Best app that you ever did saw.".to_string(),),

                neuron_minimum_dissolve_delay_to_vote_seconds: unwrap_duration_seconds(
                    &original_governance_parameters.neuron_minimum_dissolve_delay_to_vote
                ),

                initial_reward_rate_basis_points: unwrap_percentage_basis_points(
                    &original_voting_reward_parameters.initial_reward_rate
                ),
                final_reward_rate_basis_points: unwrap_percentage_basis_points(
                    &original_voting_reward_parameters.final_reward_rate
                ),
                reward_rate_transition_duration_seconds: unwrap_duration_seconds(
                    &original_voting_reward_parameters.reward_rate_transition_duration
                ),

                max_dissolve_delay_seconds: unwrap_duration_seconds(
                    &original_governance_parameters.neuron_maximum_dissolve_delay
                ),

                max_neuron_age_seconds_for_age_bonus: unwrap_duration_seconds(
                    &original_governance_parameters.neuron_maximum_age_for_age_bonus
                ),

                max_dissolve_delay_bonus_percentage: unwrap_percentage_basis_points(
                    &original_governance_parameters.neuron_maximum_dissolve_delay_bonus
                )
                .map(|basis_points| basis_points / 100),

                max_age_bonus_percentage: unwrap_percentage_basis_points(
                    &original_governance_parameters.neuron_maximum_age_bonus
                )
                .map(|basis_points| basis_points / 100),

                initial_voting_period_seconds: unwrap_duration_seconds(
                    &original_governance_parameters.proposal_initial_voting_period
                ),
                wait_for_quiet_deadline_increase_seconds: unwrap_duration_seconds(
                    &original_governance_parameters.proposal_wait_for_quiet_deadline_increase
                ),
                dapp_canisters: Some(sns_init_pb::DappCanisters {
                    canisters: vec![pb::Canister {
                        id: Some(CanisterId::from_u64(1000).get()),
                    }],
                }),
                min_participants: original_swap_parameters.minimum_participants,
                min_icp_e8s: None,
                max_icp_e8s: None,
                min_direct_participation_icp_e8s: unwrap_tokens_e8s(
                    &original_swap_parameters.minimum_direct_participation_icp
                ),
                max_direct_participation_icp_e8s: unwrap_tokens_e8s(
                    &original_swap_parameters.maximum_direct_participation_icp
                ),
                min_participant_icp_e8s: unwrap_tokens_e8s(
                    &original_swap_parameters.minimum_participant_icp
                ),
                max_participant_icp_e8s: unwrap_tokens_e8s(
                    &original_swap_parameters.maximum_participant_icp
                ),

                confirmation_text: original_swap_parameters.confirmation_text.clone(),
                restricted_countries: original_swap_parameters.restricted_countries.clone(),
                nns_proposal_id: Some(proposal_id),
                neurons_fund_participants: Some(NeuronsFundParticipants {
                    participants: vec![],
                }),
                neurons_fund_participation: Some(true),

                neurons_fund_participation_constraints: Some(
                    NEURONS_FUND_PARTICIPATION_CONSTRAINTS.clone()
                ),

                // We'll examine these later
                initial_token_distribution: None,
                neuron_basket_construction_parameters: None,
                swap_start_timestamp_seconds: None,
                swap_due_timestamp_seconds: None,
            },
        );

        let original_initial_token_distribution: &_ =
            CREATE_SERVICE_NERVOUS_SYSTEM_WITH_MATCHED_FUNDING
                .initial_token_distribution
                .as_ref()
                .unwrap();
        let original_developer_distribution: &_ = original_initial_token_distribution
            .developer_distribution
            .as_ref()
            .unwrap();
        assert_eq!(
            original_developer_distribution.developer_neurons.len(),
            1,
            "{:#?}",
            original_developer_distribution.developer_neurons,
        );
        let original_neuron_distribution: &_ = original_developer_distribution
            .developer_neurons
            .first()
            .unwrap();

        let src::SwapDistribution { total: swap_total } = original_initial_token_distribution
            .swap_distribution
            .as_ref()
            .unwrap();
        let swap_total_e8s = unwrap_tokens_e8s(swap_total).unwrap();
        assert_eq!(swap_total_e8s, 1_840_880_000);

        assert_eq!(
            converted.initial_token_distribution.unwrap(),
            sns_init_payload::InitialTokenDistribution::FractionalDeveloperVotingPower(
                sns_init_pb::FractionalDeveloperVotingPower {
                    developer_distribution: Some(sns_init_pb::DeveloperDistribution {
                        developer_neurons: vec![sns_init_pb::NeuronDistribution {
                            controller: Some(original_neuron_distribution.controller.unwrap()),

                            stake_e8s: unwrap_tokens_e8s(&original_neuron_distribution.stake)
                                .unwrap(),

                            memo: original_neuron_distribution.memo.unwrap(),

                            dissolve_delay_seconds: unwrap_duration_seconds(
                                &original_neuron_distribution.dissolve_delay
                            )
                            .unwrap(),

                            vesting_period_seconds: unwrap_duration_seconds(
                                &original_neuron_distribution.vesting_period
                            ),
                        },],
                    },),
                    treasury_distribution: Some(sns_init_pb::TreasuryDistribution {
                        total_e8s: unwrap_tokens_e8s(
                            &original_initial_token_distribution
                                .treasury_distribution
                                .as_ref()
                                .unwrap()
                                .total
                        )
                        .unwrap(),
                    },),
                    swap_distribution: Some(sns_init_pb::SwapDistribution {
                        // These are intentionally the same.
                        total_e8s: swap_total_e8s,
                        initial_swap_amount_e8s: swap_total_e8s,
                    },),
                    airdrop_distribution: Some(sns_init_pb::AirdropDistribution {
                        airdrop_neurons: vec![],
                    },),
                },
            ),
        );

        let original_neuron_basket_construction_parameters =
            CREATE_SERVICE_NERVOUS_SYSTEM_WITH_MATCHED_FUNDING
                .swap_parameters
                .as_ref()
                .unwrap()
                .neuron_basket_construction_parameters
                .as_ref()
                .unwrap();

        assert_eq!(
            converted.neuron_basket_construction_parameters.unwrap(),
            NeuronBasketConstructionParameters {
                count: original_neuron_basket_construction_parameters
                    .count
                    .unwrap(),
                dissolve_delay_interval_seconds: unwrap_duration_seconds(
                    &original_neuron_basket_construction_parameters.dissolve_delay_interval
                )
                .unwrap(),
            }
        );

        let (expected_swap_start_timestamp_seconds, expected_swap_due_timestamp_seconds) =
            CreateServiceNervousSystem::swap_start_and_due_timestamps(
                original_swap_parameters.start_time.unwrap(),
                original_swap_parameters.duration.unwrap(),
                current_timestamp_seconds,
            )
            .unwrap();

        assert_eq!(
            converted.swap_start_timestamp_seconds,
            Some(expected_swap_start_timestamp_seconds)
        );
        assert_eq!(
            converted.swap_due_timestamp_seconds,
            Some(expected_swap_due_timestamp_seconds)
        );
    }
}

mod metrics_tests {

    use maplit::btreemap;

    use crate::{
        encode_metrics,
        governance::Governance,
        pb::v1::{proposal, Governance as GovernanceProto, Motion, Proposal, ProposalData, Tally},
        test_utils::{MockEnvironment, StubCMC, StubIcpLedger},
    };

    #[test]
    fn test_metrics_total_voting_power() {
        let proposal_1 = ProposalData {
            proposal: Some(Proposal {
                title: Some("Foo Foo Bar".to_string()),
                action: Some(proposal::Action::Motion(Motion {
                    motion_text: "Text for this motion".to_string(),
                })),
                ..Proposal::default()
            }),
            latest_tally: Some(Tally {
                timestamp_seconds: 0,
                yes: 0,
                no: 0,
                total: 555,
            }),
            ..ProposalData::default()
        };

        let proposal_2 = ProposalData {
            proposal: Some(Proposal {
                title: Some("Foo Foo Bar".to_string()),
                action: Some(proposal::Action::ManageNeuron(Box::default())),

                ..Proposal::default()
            }),
            latest_tally: Some(Tally {
                timestamp_seconds: 0,
                yes: 0,
                no: 0,
                total: 1,
            }),
            ..ProposalData::default()
        };

        let governance = Governance::new(
            GovernanceProto {
                proposals: btreemap! {
                    1 =>  proposal_1,
                    2 => proposal_2
                },
                ..GovernanceProto::default()
            },
            Box::new(MockEnvironment::new(Default::default(), 0)),
            Box::new(StubIcpLedger {}),
            Box::new(StubCMC {}),
        );

        let mut writer = ic_metrics_encoder::MetricsEncoder::new(vec![], 1000);

        encode_metrics(&governance, &mut writer).unwrap();

        let body = writer.into_inner();
        let s = String::from_utf8_lossy(&body);

        // We assert that it is '555' instead of '1', so that we know the correct
        // proposal action is filtered out.
        assert!(s.contains("governance_voting_power_total 555 1000"));
    }

    #[test]
    fn test_metrics_proposal_deadline_timestamp_seconds() {
        let open_proposal = ProposalData {
            proposal: Some(Proposal {
                title: Some("open_proposal".to_string()),
                action: Some(proposal::Action::ManageNeuron(Box::default())),
                ..Proposal::default()
            }),
            ..ProposalData::default()
        };

        let rejected_proposal = ProposalData {
            proposal: Some(Proposal {
                title: Some("rejected_proposal".to_string()),
                action: Some(proposal::Action::ManageNeuron(Box::default())),
                ..Proposal::default()
            }),
            decided_timestamp_seconds: 1,
            ..ProposalData::default()
        };

        let governance = Governance::new(
            GovernanceProto {
                proposals: btreemap! {
                    1 =>  open_proposal.clone(),
                    2 =>  rejected_proposal,
                },
                ..GovernanceProto::default()
            },
            Box::<MockEnvironment>::default(),
            Box::new(StubIcpLedger {}),
            Box::new(StubCMC {}),
        );

        let mut writer = ic_metrics_encoder::MetricsEncoder::new(vec![], 10);

        encode_metrics(&governance, &mut writer).unwrap();

        let voting_period = governance.voting_period_seconds()(open_proposal.topic());
        let deadline_ts = open_proposal.get_deadline_timestamp_seconds(voting_period);
        let body = writer.into_inner();
        let s = String::from_utf8_lossy(&body);

        assert!(s.contains(&format!(
            "governance_proposal_deadline_timestamp_seconds{{proposal_id=\"1\"}} {} 10",
            deadline_ts
        )));

        // We assert that decided proposals are filtered out from metrics
        assert!(!s.contains("governance_proposal_deadline_timestamp_seconds{proposal_id=\"2\"}"));
    }
}

mod neuron_archiving_tests {
    use crate::pb::v1::{neuron::DissolveState, Neuron};
    use proptest::proptest;

    #[test]
    fn test_neuron_is_inactive_based_on_neurons_fund_membership() {
        const NOW: u64 = 123_456_789;

        // Dissolved in the distant past.
        let model_neuron = Neuron {
            dissolve_state: Some(DissolveState::WhenDissolvedTimestampSeconds(42)),
            ..Default::default()
        };
        assert!(model_neuron.is_inactive(NOW), "{:#?}", model_neuron);

        // Case Some(positive): Active.
        let neuron = Neuron {
            joined_community_fund_timestamp_seconds: Some(42),
            ..model_neuron.clone()
        };
        assert!(!neuron.is_inactive(NOW), "{:#?}", neuron);

        // Case Some(0): Inactive.
        let neuron = Neuron {
            joined_community_fund_timestamp_seconds: Some(0),
            ..model_neuron.clone()
        };
        assert!(neuron.is_inactive(NOW), "{:#?}", neuron);

        // Case None: Same as Some(0), i.e. Inactive
        let neuron = Neuron {
            joined_community_fund_timestamp_seconds: None,
            ..model_neuron.clone()
        };
        assert!(neuron.is_inactive(NOW), "{:#?}", neuron);

        // This is just so that clone is always called in all of the above cases.
        drop(model_neuron);
    }

    #[test]
    fn test_neuron_is_inactive_based_on_dissolve_state() {
        const NOW: u64 = 123_456_789;

        // Case 0: None: Active
        let neuron = Neuron::default();
        assert!(!neuron.is_inactive(NOW), "{:#?}", neuron);

        // Case 1a: Dissolved in the "distant" past: Inactive. This is the only case where
        // "inactive" is the expected result.
        let neuron = Neuron {
            dissolve_state: Some(DissolveState::WhenDissolvedTimestampSeconds(42)),
            ..Default::default()
        };
        assert!(neuron.is_inactive(NOW), "{:#?}", neuron);

        // Case 1b: Dissolved right now: Active
        let neuron = Neuron {
            dissolve_state: Some(DissolveState::WhenDissolvedTimestampSeconds(NOW)),
            ..Default::default()
        };
        assert!(!neuron.is_inactive(NOW), "{:#?}", neuron);

        // Case 1c: Dissolved right now: Active (again).
        let neuron = Neuron {
            dissolve_state: Some(DissolveState::WhenDissolvedTimestampSeconds(NOW + 42)),
            ..Default::default()
        };
        assert!(!neuron.is_inactive(NOW), "{:#?}", neuron);

        // Case 2a: DissolveDelay(0): Active
        let neuron = Neuron {
            dissolve_state: Some(DissolveState::DissolveDelaySeconds(0)),
            ..Default::default()
        };
        assert!(!neuron.is_inactive(NOW), "{:#?}", neuron);

        // Case 2b: DissolveDelay(positive): Active
        let neuron = Neuron {
            dissolve_state: Some(DissolveState::DissolveDelaySeconds(42)),
            ..Default::default()
        };
        assert!(!neuron.is_inactive(NOW), "{:#?}", neuron);
    }

    proptest! {
        #[test]
        fn test_neuron_is_inactive_based_on_funding(
            cached_neuron_stake_e8s in 0_u64..10,
            staked_maturity_e8s_equivalent in 0_u64..10,
            neuron_fees_e8s in 0_u64..10,
            maturity_e8s_equivalent in 0_u64..10,
        ) {
            let net_funding_e8s = (
                cached_neuron_stake_e8s
                    .saturating_sub(neuron_fees_e8s)
                    .saturating_add(staked_maturity_e8s_equivalent)
            )
            + maturity_e8s_equivalent;
            let is_funded = net_funding_e8s > 0;

            // The test subject will be WhenDissolved(reasonable_time). Therefore, by living in the
            // distant future, the test subject will be considered "dissolved in the sufficiently
            // distant past". Thus, the dissolve_state requirement to be "inactive" is met.
            let now = 123_456_789;

            let staked_maturity_e8s_equivalent = Some(staked_maturity_e8s_equivalent);
            let neuron = Neuron {
                cached_neuron_stake_e8s,
                staked_maturity_e8s_equivalent,
                neuron_fees_e8s,
                maturity_e8s_equivalent,

                dissolve_state: Some(DissolveState::WhenDissolvedTimestampSeconds(42)),

                ..Default::default()
            };

            assert_eq!(
                neuron.is_inactive(now),
                !is_funded,
                "cached stake: {cached_neuron_stake_e8s}\n\
                 staked maturity: {staked_maturity_e8s_equivalent:?}\n\
                 fees: {neuron_fees_e8s}\n\
                 maturity: {maturity_e8s_equivalent}\n\
                 net funding: {net_funding_e8s}\n\
                 Neuron:\n{neuron:#?}",
            );
        }
    } // end proptest
}

mod cast_vote_and_cascade_follow {
    use crate::{
        governance::{Governance, MIN_DISSOLVE_DELAY_FOR_VOTE_ELIGIBILITY_SECONDS},
        neuron_store::NeuronStore,
        pb::v1::{
            neuron::{DissolveState, Followees},
            Ballot, Neuron, Topic, Vote,
        },
    };
    use ic_nns_common::pb::v1::{NeuronId, ProposalId};
    use maplit::hashmap;
    use std::collections::{BTreeMap, HashMap};

    const E8S: u64 = 100_000_000;

    fn make_ballot(voting_power: u64, vote: Vote) -> Ballot {
        Ballot {
            voting_power,
            vote: vote as i32,
        }
    }

    fn make_test_neuron_with_followees(
        id: u64,
        topic: Topic,
        followees: Vec<u64>,
        aging_since_timestamp_seconds: u64,
    ) -> Neuron {
        Neuron {
            id: Some(NeuronId { id }),
            followees: hashmap! {
                topic as i32 => Followees {
                    followees: followees.into_iter().map(|id| NeuronId { id }).collect()
                }
            },
            cached_neuron_stake_e8s: E8S, // clippy doesn't like 1 * E8S
            dissolve_state: Some(DissolveState::DissolveDelaySeconds(
                MIN_DISSOLVE_DELAY_FOR_VOTE_ELIGIBILITY_SECONDS,
            )),
            aging_since_timestamp_seconds,
            ..Default::default()
        }
    }

    #[test]
    fn test_cast_vote_and_cascade_doesnt_cascade_neuron_management() {
        let now = 1000;
        let topic = Topic::NeuronManagement;

        let make_neuron = |id: u64, followees: Vec<u64>| {
            make_test_neuron_with_followees(id, topic, followees, now)
        };

        let add_neuron_with_ballot = |neuron_map: &mut BTreeMap<u64, Neuron>,
                                      ballots: &mut HashMap<u64, Ballot>,
                                      id: u64,
                                      followees: Vec<u64>,
                                      vote: Vote| {
            let neuron = make_neuron(id, followees);
            let voting_power = neuron.voting_power(now);
            neuron_map.insert(id, neuron);
            ballots.insert(id, make_ballot(voting_power, vote));
        };

        let add_neuron_without_ballot =
            |neuron_map: &mut BTreeMap<u64, Neuron>, id: u64, followees: Vec<u64>| {
                let neuron = make_neuron(id, followees);
                neuron_map.insert(id, neuron);
            };

        let mut heap_neurons = BTreeMap::new();
        let mut ballots = HashMap::new();
        for id in 1..=5 {
            // Each neuron follows all neurons with a lower id
            let followees = (1..id).collect();

            add_neuron_with_ballot(
                &mut heap_neurons,
                &mut ballots,
                id,
                followees,
                Vote::Unspecified,
            );
        }
        // Add another neuron that follows both a neuron with a ballot and without a ballot
        add_neuron_with_ballot(
            &mut heap_neurons,
            &mut ballots,
            6,
            vec![1, 7],
            Vote::Unspecified,
        );

        // Add a neuron without a ballot for neuron 6 to follow.
        add_neuron_without_ballot(&mut heap_neurons, 7, vec![1]);

        let mut neuron_store = NeuronStore::new(heap_neurons);

        Governance::cast_vote_and_cascade_follow(
            &ProposalId { id: 1 },
            &mut ballots,
            &NeuronId { id: 1 },
            Vote::Yes,
            topic,
            &mut neuron_store,
        );

        assert_eq!(
            ballots,
            hashmap! {
                1 => make_ballot(neuron_store.with_neuron(&NeuronId {id: 1}, |n| n.voting_power(now)).unwrap(), Vote::Yes),
                2 => make_ballot(neuron_store.with_neuron(&NeuronId {id: 2}, |n| n.voting_power(now)).unwrap(), Vote::Unspecified),
                3 => make_ballot(neuron_store.with_neuron(&NeuronId {id: 3}, |n| n.voting_power(now)).unwrap(), Vote::Unspecified),
                4 => make_ballot(neuron_store.with_neuron(&NeuronId {id: 4}, |n| n.voting_power(now)).unwrap(), Vote::Unspecified),
                5 => make_ballot(neuron_store.with_neuron(&NeuronId {id: 5}, |n| n.voting_power(now)).unwrap(), Vote::Unspecified),
                6 => make_ballot(neuron_store.with_neuron(&NeuronId {id: 6}, |n| n.voting_power(now)).unwrap(), Vote::Unspecified),
            }
        );
    }

    #[test]
    fn test_cast_vote_and_cascade_works() {
        let now = 1000;
        let topic = Topic::NetworkCanisterManagement;

        let make_neuron = |id: u64, followees: Vec<u64>| {
            make_test_neuron_with_followees(id, topic, followees, now)
        };

        let add_neuron_with_ballot = |neuron_map: &mut BTreeMap<u64, Neuron>,
                                      ballots: &mut HashMap<u64, Ballot>,
                                      id: u64,
                                      followees: Vec<u64>,
                                      vote: Vote| {
            let neuron = make_neuron(id, followees);
            let voting_power = neuron.voting_power(now);
            neuron_map.insert(id, neuron);
            ballots.insert(id, make_ballot(voting_power, vote));
        };

        let add_neuron_without_ballot =
            |neuron_map: &mut BTreeMap<u64, Neuron>, id: u64, followees: Vec<u64>| {
                let neuron = make_neuron(id, followees);
                neuron_map.insert(id, neuron);
            };

        let mut neurons = BTreeMap::new();
        let mut ballots = HashMap::new();
        for id in 1..=5 {
            // Each neuron follows all neurons with a lower id
            let followees = (1..id).collect();

            add_neuron_with_ballot(&mut neurons, &mut ballots, id, followees, Vote::Unspecified);
        }
        // Add another neuron that follows both a neuron with a ballot and without a ballot
        add_neuron_with_ballot(&mut neurons, &mut ballots, 6, vec![1, 7], Vote::Unspecified);

        // Add a neuron without a ballot for neuron 6 to follow.
        add_neuron_without_ballot(&mut neurons, 7, vec![1]);

        let mut neuron_store = NeuronStore::new(neurons);

        Governance::cast_vote_and_cascade_follow(
            &ProposalId { id: 1 },
            &mut ballots,
            &NeuronId { id: 1 },
            Vote::Yes,
            topic,
            &mut neuron_store,
        );

        assert_eq!(
            ballots,
            hashmap! {
                1 => make_ballot(neuron_store.with_neuron(&NeuronId {id: 1}, |n| n.voting_power(now)).unwrap(), Vote::Yes),
                2 => make_ballot(neuron_store.with_neuron(&NeuronId {id: 2}, |n| n.voting_power(now)).unwrap(), Vote::Yes),
                3 => make_ballot(neuron_store.with_neuron(&NeuronId {id: 3}, |n| n.voting_power(now)).unwrap(), Vote::Yes),
                4 => make_ballot(neuron_store.with_neuron(&NeuronId {id: 4}, |n| n.voting_power(now)).unwrap(), Vote::Yes),
                5 => make_ballot(neuron_store.with_neuron(&NeuronId {id: 5}, |n| n.voting_power(now)).unwrap(), Vote::Yes),
                6 => make_ballot(neuron_store.with_neuron(&NeuronId {id: 6}, |n| n.voting_power(now)).unwrap(), Vote::Unspecified),
            }
        );
    }
}

#[test]
fn governance_remove_neuron_updates_followee_index_correctly() {
    let mut governance = Governance::new(
        GovernanceProto {
            neurons: btreemap! {
                1 => Neuron {
                    id: Some(NeuronId { id: 1 }),
                    followees: hashmap! {
                         2 => Followees {
                            followees: vec![NeuronId { id: 2 }, NeuronId { id: 3 }]
                        }
                    },
                    ..Default::default()
                },
            },
            ..Default::default()
        },
        Box::new(MockEnvironment::new(vec![], 0)),
        Box::new(StubIcpLedger {}),
        Box::new(StubCMC {}),
    );

    let entry = governance
        .neuron_store
        .get_followers_by_followee_and_topic(NeuronId { id: 2 }, Topic::try_from(2).unwrap());
    assert_eq!(entry, vec![NeuronId { id: 1 }]);

    let neuron = governance
        .with_neuron(&NeuronId { id: 1 }, |n| n.clone())
        .unwrap();
    governance.remove_neuron(neuron).unwrap();

    let entry = governance
        .neuron_store
        .get_followers_by_followee_and_topic(NeuronId { id: 2 }, Topic::try_from(2).unwrap());
    assert_eq!(entry, vec![]);
}

#[test]
fn test_pre_and_post_upgrade_first_time() {
    let neuron1 = Neuron {
        id: Some(NeuronId { id: 1 }),
        followees: hashmap! {
            2 => Followees {
                followees: vec![NeuronId { id : 3}]
            }
        },
        account: vec![0; 32],
        ..Default::default()
    };
    let neurons = btreemap! { 1 => neuron1 };

    // This simulates the state of heap on first post_upgrade (empty topic_followee_index)
    let governance_proto = GovernanceProto {
        neurons,
        ..Default::default()
    };

    // Precondition
    assert_eq!(governance_proto.neurons.len(), 1);
    assert_eq!(governance_proto.topic_followee_index.len(), 0);

    // Then Governance is instantiated during upgrade with proto
    let mut governance = Governance::new(
        governance_proto,
        Box::<MockEnvironment>::default(),
        Box::new(StubIcpLedger {}),
        Box::new(StubCMC {}),
    );
    // On next pre-upgrade, we get the heap proto and store it in stable memory
    let mut extracted_proto = governance.take_heap_proto();

    // topic_followee_index should have been populated
    assert_eq!(extracted_proto.topic_followee_index.len(), 1);

    // We now modify it so that we can be assured that it is not rebuilding on the next post_upgrade
    extracted_proto.topic_followee_index.insert(
        4,
        FollowersMap {
            followers_map: hashmap! {5 => Followers { followers: vec![NeuronId { id : 6}]}},
        },
    );

    assert_eq!(extracted_proto.neurons.len(), 1);
    assert_eq!(extracted_proto.topic_followee_index.len(), 2);

    // We now simulate the post_upgrade
    let mut governance = Governance::new_restored(
        extracted_proto,
        Box::<MockEnvironment>::default(),
        Box::new(StubIcpLedger {}),
        Box::new(StubCMC {}),
        None,
    );

    // It should not rebuild during post_upgrade so it should still be mis-matched with neurons.
    let extracted_proto = governance.take_heap_proto();
    assert_eq!(extracted_proto.topic_followee_index.len(), 2);
}

#[test]
fn can_spawn_neurons_only_true_when_not_spawning_and_neurons_ready_to_spawn() {
    let proto = GovernanceProto {
        ..Default::default()
    };

    let mock_env = MockEnvironment::new(vec![], 100);

    let mut governance = Governance::new(
        proto,
        Box::new(mock_env),
        Box::new(StubIcpLedger {}),
        Box::new(StubCMC {}),
    );
    // No neurons to spawn...
    assert!(!governance.can_spawn_neurons());

    governance
        .neuron_store
        .add_neuron(Neuron {
            id: Some(NeuronId { id: 1 }),
            spawn_at_timestamp_seconds: Some(99),
            ..Default::default()
        })
        .unwrap();

    governance.heap_data.spawning_neurons = Some(true);

    // spawning_neurons is true, so it shouldn't be able to spawn again.
    assert!(!governance.can_spawn_neurons());

    governance.heap_data.spawning_neurons = None;

    // Work to do, no lock, should say yes.
    assert!(governance.can_spawn_neurons());
}

#[test]
fn topic_min_max_test() {
    use strum::IntoEnumIterator;

    for topic in Topic::iter() {
        assert!(topic >= Topic::MIN, "Topic::MIN needs to be updated");
        assert!(topic <= Topic::MAX, "Topic::MAX needs to be updated");
    }
}
