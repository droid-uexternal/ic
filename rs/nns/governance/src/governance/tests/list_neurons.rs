use crate::{
    governance::Governance,
    pb::v1::{neuron::DissolveState, NetworkEconomics, Neuron},
    test_utils::{MockEnvironment, StubCMC, StubIcpLedger},
};
use ic_base_types::PrincipalId;
use ic_nns_common::pb::v1::NeuronId;
use ic_nns_governance_api::pb::v1::ListNeurons;

#[test]
fn test_list_neurons_with_paging() {
    let user_id = PrincipalId::new_user_test_id(100);

    let neurons = (1..1000u64)
        .map(|id| {
            let dissolve_state = DissolveState::DissolveDelaySeconds(100);
            let account = crate::test_utils::test_subaccount_for_neuron_id(id);
            (
                id,
                Neuron {
                    id: Some(NeuronId::from_u64(id)),
                    controller: Some(user_id),
                    account,
                    dissolve_state: Some(dissolve_state),
                    // Fill in the rest as needed (stake, maturity, etc.)
                    ..Default::default()
                },
            )
        })
        .collect();

    let governance = Governance::new(
        crate::pb::v1::Governance {
            neurons,
            economics: Some(NetworkEconomics {
                voting_power_economics: Some(Default::default()),
                ..Default::default()
            }),
            ..crate::pb::v1::Governance::default()
        },
        Box::new(MockEnvironment::new(Default::default(), 0)),
        Box::new(StubIcpLedger {}),
        Box::new(StubCMC {}),
    );

    let mut request = ListNeurons {
        neuron_ids: vec![],
        include_neurons_readable_by_caller: true,
        include_empty_neurons_readable_by_caller: Some(true),
        include_public_neurons_in_full_neurons: None,
        page_number: None,
        page_size: None,
    };

    let response_with_no_page_number = governance.list_neurons(&request, user_id);
    request.page_number = Some(0);
    let response_with_0_page_number = governance.list_neurons(&request, user_id);

    assert_eq!(response_with_0_page_number, response_with_no_page_number);
    assert_eq!(response_with_0_page_number.full_neurons.len(), 500);
    assert_eq!(response_with_0_page_number.total_pages_available, Some(2));

    let response = governance.list_neurons(
        &ListNeurons {
            neuron_ids: vec![],
            include_neurons_readable_by_caller: true,
            include_empty_neurons_readable_by_caller: Some(true),
            include_public_neurons_in_full_neurons: None,
            page_number: Some(1),
            page_size: None,
        },
        user_id,
    );

    assert_eq!(response.full_neurons.len(), 499);
    assert_eq!(response.total_pages_available, Some(2));

    // Assert maximum page size cannot be exceeded
    let response = governance.list_neurons(
        &ListNeurons {
            neuron_ids: vec![],
            include_neurons_readable_by_caller: true,
            include_empty_neurons_readable_by_caller: Some(true),
            include_public_neurons_in_full_neurons: None,
            page_number: Some(0),
            page_size: Some(501),
        },
        user_id,
    );

    assert_eq!(response.full_neurons.len(), 500);
    assert_eq!(response.total_pages_available, Some(2));
}
