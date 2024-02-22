use crate::flow::{
    ApprovalFlow, DepositFlow, DepositParams, LedgerTransactionAssert, WithdrawalFlow,
};
use crate::mock::JsonRpcMethod;
use candid::{Decode, Encode, Nat, Principal};
use ic_canisters_http_types::{HttpRequest, HttpResponse};
use ic_cketh_minter::endpoints::events::{Event, EventPayload, GetEventsResult};
use ic_cketh_minter::endpoints::{AddCkErc20Token, MinterInfo, RetrieveEthStatus, WithdrawalArg};
use ic_cketh_minter::lifecycle::upgrade::UpgradeArg;
use ic_cketh_minter::logs::Log;
use ic_cketh_minter::{
    endpoints::{CandidBlockTag, Eip1559TransactionPrice},
    lifecycle::{init::InitArg as MinterInitArgs, EthereumNetwork, MinterArg},
};
use ic_ethereum_types::Address;
use ic_icrc1_ledger::{InitArgsBuilder as LedgerInitArgsBuilder, LedgerArgument};
use ic_state_machine_tests::{
    CanisterId, Cycles, PrincipalId, StateMachine, StateMachineBuilder, UserError, WasmResult,
};
use ic_test_utilities_load_wasm::load_wasm;
use icrc_ledger_types::icrc1::account::Account;
use icrc_ledger_types::icrc2::approve::{ApproveArgs, ApproveError};
use num_traits::cast::ToPrimitive;
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::str::FromStr;
use std::time::Duration;

pub mod flow;
pub mod mock;
pub mod response;
#[cfg(test)]
mod tests;

pub const CKETH_TRANSFER_FEE: u64 = 10;
pub const MAX_TICKS: usize = 10;
pub const DEFAULT_PRINCIPAL_ID: u64 = 10352385;
pub const DEFAULT_DEPOSIT_BLOCK_NUMBER: u64 = 0x9;
pub const DEFAULT_DEPOSIT_FROM_ADDRESS: &str = "0x55654e7405fcb336386ea8f36954a211b2cda764";
pub const DEFAULT_DEPOSIT_TRANSACTION_HASH: &str =
    "0xcfa48c44dc89d18a898a42b4a5b02b6847a3c2019507d5571a481751c7a2f353";
pub const DEFAULT_DEPOSIT_LOG_INDEX: u64 = 0x24;
pub const DEFAULT_BLOCK_HASH: &str =
    "0x82005d2f17b251900968f01b0ed482cb49b7e1d797342bc504904d442b64dbe4";
pub const LAST_SCRAPED_BLOCK_NUMBER_AT_INSTALL: u64 = 3_956_206;
pub const DEFAULT_BLOCK_NUMBER: u64 = 0x4132ec;
pub const EXPECTED_BALANCE: u64 = 100_000_000_000_000_000;
pub const EFFECTIVE_GAS_PRICE: u64 = 4_277_923_390;

pub const DEFAULT_WITHDRAWAL_TRANSACTION_HASH: &str =
    "0x2cf1763e8ee3990103a31a5709b17b83f167738abb400844e67f608a98b0bdb5";
pub const DEFAULT_WITHDRAWAL_TRANSACTION: &str = "0x02f87301808459682f008507af2c9f6282520894221e931fbfcb9bd54ddd26ce6f5e29e98add01c0880160cf1e9917a0e680c001a0b27af25a08e87836a778ac2858fdfcff1f6f3a0d43313782c81d05ca34b80271a078026b399a32d3d7abab625388a3c57f651c66a182eb7f8b1a58d9aef7547256";
pub const MINTER_ADDRESS: &str = "0xfd644a761079369962386f8e4259217c2a10b8d0";
pub const DEFAULT_WITHDRAWAL_DESTINATION_ADDRESS: &str =
    "0x221E931fbFcb9bd54DdD26cE6f5e29E98AdD01C0";
pub const HELPER_SMART_CONTRACT_ADDRESS: &str = "0x907b6efc1a398fd88a8161b3ca02eec8eaf72ca1";
pub const RECEIVED_ETH_EVENT_TOPIC: &str =
    "0x257e057bb61920d8d0ed2cb7b720ac7f9c513cd1110bc9fa543079154f45f435";
pub const HEADER_SIZE_LIMIT: u64 = 2 * 1024;
pub const MAX_ETH_LOGS_BLOCK_RANGE: u64 = 799;

pub struct CkEthSetup {
    pub env: StateMachine,
    pub caller: PrincipalId,
    pub ledger_id: CanisterId,
    pub minter_id: CanisterId,
}

impl Default for CkEthSetup {
    fn default() -> Self {
        Self::new()
    }
}

impl CkEthSetup {
    pub fn new() -> Self {
        let env = StateMachineBuilder::new()
            .with_default_canister_range()
            .build();
        let minter_id =
            env.create_canister_with_cycles(None, Cycles::new(100_000_000_000_000), None);
        let ledger_id = env.create_canister(None);

        env.install_existing_canister(
            ledger_id,
            ledger_wasm(),
            Encode!(&LedgerArgument::Init(
                LedgerInitArgsBuilder::with_symbol_and_name("ckETH", "ckETH")
                    .with_minting_account(minter_id.get().0)
                    .with_transfer_fee(CKETH_TRANSFER_FEE)
                    .with_max_memo_length(80)
                    .with_decimals(18)
                    .with_feature_flags(ic_icrc1_ledger::FeatureFlags { icrc2: true })
                    .build(),
            ))
            .unwrap(),
        )
        .unwrap();
        let minter_id = install_minter(&env, ledger_id, minter_id);
        let caller = PrincipalId::new_user_test_id(DEFAULT_PRINCIPAL_ID);

        let cketh = Self {
            env,
            caller,
            ledger_id,
            minter_id,
        };

        assert_eq!(
            Address::from_str(MINTER_ADDRESS).unwrap(),
            Address::from_str(&cketh.minter_address()).unwrap()
        );
        cketh
    }

    pub fn deposit(self, params: DepositParams) -> DepositFlow {
        DepositFlow {
            setup: self,
            params,
        }
    }

    pub fn minter_address(&self) -> String {
        Decode!(
            &assert_reply(
                self.env
                    .execute_ingress_as(
                        self.caller,
                        self.minter_id,
                        "minter_address",
                        Encode!().unwrap(),
                    )
                    .expect("failed to get eth address")
            ),
            String
        )
        .unwrap()
    }

    pub fn retrieve_eth_status(&self, block_index: &Nat) -> RetrieveEthStatus {
        Decode!(
            &assert_reply(
                self.env
                    .execute_ingress_as(
                        self.caller,
                        self.minter_id,
                        "retrieve_eth_status",
                        Encode!(&block_index.0.to_u64().unwrap()).unwrap(),
                    )
                    .expect("failed to get eth address")
            ),
            RetrieveEthStatus
        )
        .unwrap()
    }

    pub fn balance_of(&self, account: impl Into<Account>) -> Nat {
        Decode!(
            &assert_reply(
                self.env
                    .query(
                        self.ledger_id,
                        "icrc1_balance_of",
                        Encode!(&account.into()).unwrap()
                    )
                    .expect("failed to query balance on the ledger")
            ),
            Nat
        )
        .unwrap()
    }

    pub fn eip_1559_transaction_price(
        &self,
    ) -> Result<WasmResult, ic_state_machine_tests::UserError> {
        self.env.query(
            self.minter_id,
            "eip_1559_transaction_price",
            Encode!().unwrap(),
        )
    }

    pub fn eip_1559_transaction_price_expecting_ok(&self) -> Eip1559TransactionPrice {
        Decode!(
            &assert_reply(self.eip_1559_transaction_price().unwrap()),
            Eip1559TransactionPrice
        )
        .unwrap()
    }

    pub fn add_ckerc20_token(
        &self,
        from: Principal,
        erc20: &AddCkErc20Token,
    ) -> Result<WasmResult, UserError> {
        self.env.execute_ingress_as(
            PrincipalId::from(from),
            self.minter_id,
            "add_ckerc20_token",
            Encode!(erc20).unwrap(),
        )
    }

    pub fn add_ckerc20_token_expecting_ok(self, from: Principal, erc20: &AddCkErc20Token) -> Self {
        Decode!(
            &assert_reply(self.add_ckerc20_token(from, erc20).unwrap()),
            ()
        )
        .unwrap();
        self
    }

    pub fn get_minter_info(&self) -> MinterInfo {
        Decode!(
            &assert_reply(
                self.env
                    .query(self.minter_id, "get_minter_info", Encode!().unwrap())
                    .unwrap()
            ),
            MinterInfo
        )
        .unwrap()
    }

    pub fn call_ledger_approve_minter(
        self,
        from: Principal,
        amount: u64,
        from_subaccount: Option<[u8; 32]>,
    ) -> ApprovalFlow {
        let approval_response = Decode!(&assert_reply(self.env.execute_ingress_as(
            PrincipalId::from(from),
            self.ledger_id,
            "icrc2_approve",
            Encode!(&ApproveArgs {
                from_subaccount,
                spender: Account {
                    owner: self.minter_id.into(),
                    subaccount: None
                },
                amount: Nat::from(amount),
                expected_allowance: None,
                expires_at: None,
                fee: None,
                memo: None,
                created_at_time: None,
            }).unwrap()
            ).expect("failed to execute token transfer")),
            Result<Nat, ApproveError>
        )
        .unwrap();
        ApprovalFlow {
            setup: self,
            approval_response,
        }
    }

    pub fn call_ledger_get_transaction<T: Into<Nat>>(
        self,
        ledger_index: T,
    ) -> LedgerTransactionAssert {
        use icrc_ledger_types::icrc3::transactions::{
            GetTransactionsRequest, GetTransactionsResponse,
        };

        let request = GetTransactionsRequest {
            start: ledger_index.into(),
            length: 1_u8.into(),
        };
        let mut response = Decode!(
            &assert_reply(
                self.env
                    .query(
                        self.ledger_id,
                        "get_transactions",
                        Encode!(&request).unwrap()
                    )
                    .expect("failed to query get_transactions on the ledger")
            ),
            GetTransactionsResponse
        )
        .unwrap();
        assert_eq!(
            response.transactions.len(),
            1,
            "Expected exactly one transaction but got {:?}",
            response.transactions
        );
        LedgerTransactionAssert {
            setup: self,
            ledger_transaction: response.transactions.pop().unwrap(),
        }
    }

    pub fn call_minter_withdraw_eth(
        self,
        from: Principal,
        amount: Nat,
        recipient: String,
    ) -> WithdrawalFlow {
        let arg = WithdrawalArg { amount, recipient };
        let message_id = self.env.send_ingress(
            PrincipalId::from(from),
            self.minter_id,
            "withdraw_eth",
            Encode!(&arg).expect("failed to encode withdraw args"),
        );
        WithdrawalFlow {
            setup: self,
            message_id,
        }
    }

    pub fn _get_logs(&self, priority: &str) -> Log {
        let request = HttpRequest {
            method: "".to_string(),
            url: format!("/logs?priority={priority}"),
            headers: vec![],
            body: serde_bytes::ByteBuf::new(),
        };
        let response = Decode!(
            &assert_reply(
                self.env
                    .query(self.minter_id, "http_request", Encode!(&request).unwrap(),)
                    .expect("failed to get minter info")
            ),
            HttpResponse
        )
        .unwrap();
        serde_json::from_slice(&response.body).expect("failed to parse ckbtc minter log")
    }

    pub fn assert_has_unique_events_in_order(self, expected_events: &[EventPayload]) -> Self {
        let audit_events = self.get_all_events();
        let mut found_event_indexes = BTreeMap::new();
        for (index_expected_event, expected_event) in expected_events.iter().enumerate() {
            for (index_audit_event, audit_event) in audit_events.iter().enumerate() {
                if &audit_event.payload == expected_event {
                    assert_eq!(
                        found_event_indexes.insert(index_expected_event, index_audit_event),
                        None,
                        "Event {:?} occurs multiple times",
                        expected_event
                    );
                }
            }
            assert!(
                found_event_indexes.contains_key(&index_expected_event),
                "Missing event {:?}",
                expected_event
            )
        }
        let audit_event_indexes = found_event_indexes.into_values().collect::<Vec<_>>();
        let sorted_audit_event_indexes = {
            let mut indexes = audit_event_indexes.clone();
            indexes.sort_unstable();
            indexes
        };
        assert_eq!(
            audit_event_indexes, sorted_audit_event_indexes,
            "Events were found in unexpected order"
        );
        self
    }

    pub fn assert_has_no_event_satisfying<P: Fn(&EventPayload) -> bool>(
        self,
        predicate: P,
    ) -> Self {
        if let Some(unexpected_event) = self
            .get_all_events()
            .into_iter()
            .find(|event| predicate(&event.payload))
        {
            panic!(
                "Found an event satisfying the predicate: {:?}",
                unexpected_event
            )
        }
        self
    }

    fn get_events(&self, start: u64, length: u64) -> GetEventsResult {
        use ic_cketh_minter::endpoints::events::GetEventsArg;

        Decode!(
            &assert_reply(
                self.env
                    .execute_ingress(
                        self.minter_id,
                        "get_events",
                        Encode!(&GetEventsArg { start, length }).unwrap(),
                    )
                    .expect("failed to get minter info")
            ),
            GetEventsResult
        )
        .unwrap()
    }

    pub fn get_all_events(&self) -> Vec<Event> {
        const FIRST_BATCH_SIZE: u64 = 100;
        let GetEventsResult {
            mut events,
            total_event_count,
        } = self.get_events(0, FIRST_BATCH_SIZE);
        while events.len() < total_event_count as usize {
            let mut next_batch =
                self.get_events(events.len() as u64, total_event_count - events.len() as u64);
            events.append(&mut next_batch.events);
        }
        events
    }

    fn check_audit_log(&self) {
        Decode!(
            &assert_reply(
                self.env
                    .query(self.minter_id, "check_audit_log", Encode!().unwrap())
                    .unwrap(),
            ),
            ()
        )
        .unwrap()
    }

    fn upgrade_minter(&self, upgrade_arg: UpgradeArg) {
        self.env
            .upgrade_canister(
                self.minter_id,
                minter_wasm(),
                Encode!(&MinterArg::UpgradeArg(upgrade_arg)).unwrap(),
            )
            .unwrap();
    }

    pub fn upgrade_minter_to_add_orchestrator_id(self, orchestrator_id: Principal) -> Self {
        self.upgrade_minter(UpgradeArg {
            next_transaction_nonce: None,
            minimum_withdrawal_amount: None,
            ethereum_contract_address: None,
            ethereum_block_height: None,
            ledger_suite_orchestrator_id: Some(orchestrator_id),
        });
        self
    }

    pub fn check_audit_logs_and_upgrade(self, upgrade_arg: UpgradeArg) -> Self {
        self.check_audit_log();
        self.env.tick(); //tick before upgrade to finish current timers which are reset afterwards
        self.upgrade_minter(upgrade_arg);
        self
    }

    pub fn assert_has_no_rpc_call(self, method: &JsonRpcMethod) -> Self {
        for _ in 0..MAX_TICKS {
            if let Some(unexpected_request) = self
                .env
                .canister_http_request_contexts()
                .values()
                .map(|context| {
                    crate::mock::JsonRpcRequest::from_str(
                        std::str::from_utf8(&context.body.clone().unwrap()).unwrap(),
                    )
                    .expect("BUG: invalid JSON RPC method")
                })
                .find(|rpc_request| rpc_request.method.to_string() == method.to_string())
            {
                panic!("Unexpected RPC call: {:?}", unexpected_request);
            }
            self.env.tick();
            self.env.advance_time(Duration::from_nanos(1));
        }
        self
    }
}

fn ledger_wasm() -> Vec<u8> {
    let path = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap())
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("rosetta-api")
        .join("icrc1")
        .join("ledger");
    load_wasm(path, "ic-icrc1-ledger", &[])
}

fn minter_wasm() -> Vec<u8> {
    load_wasm(
        std::env::var("CARGO_MANIFEST_DIR").unwrap(),
        "cketh_minter",
        &[],
    )
}

fn install_minter(env: &StateMachine, ledger_id: CanisterId, minter_id: CanisterId) -> CanisterId {
    let args = MinterInitArgs {
        ecdsa_key_name: "master_ecdsa_public_key".parse().unwrap(),
        ethereum_network: EthereumNetwork::Mainnet,
        ledger_id: ledger_id.get().0,
        next_transaction_nonce: 0_u8.into(),
        ethereum_block_height: CandidBlockTag::Finalized,
        ethereum_contract_address: Some(HELPER_SMART_CONTRACT_ADDRESS.to_string()),
        minimum_withdrawal_amount: CKETH_TRANSFER_FEE.into(),
        last_scraped_block_number: LAST_SCRAPED_BLOCK_NUMBER_AT_INSTALL.into(),
    };
    let minter_arg = MinterArg::InitArg(args);
    env.install_existing_canister(minter_id, minter_wasm(), Encode!(&minter_arg).unwrap())
        .unwrap();
    minter_id
}

fn assert_reply(result: WasmResult) -> Vec<u8> {
    match result {
        WasmResult::Reply(bytes) => bytes,
        WasmResult::Reject(reject) => {
            panic!("Expected a successful reply, got a reject: {}", reject)
        }
    }
}
