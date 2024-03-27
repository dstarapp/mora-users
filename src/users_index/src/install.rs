use candid::{CandidType, Encode, Nat};
use ic_cdk::api::call::CallResult;
use ic_cdk::export::{candid, Principal};
use ic_cdk::print;
use serde::{Deserialize, Serialize};

#[derive(CandidType, Deserialize)]
pub enum InstallMode {
    #[serde(rename = "install")]
    Install,
    #[serde(rename = "reinstall")]
    Reinstall,
    #[serde(rename = "upgrade")]
    Upgrade,
}

#[derive(CandidType, Clone, Deserialize, Debug)]
pub struct CanisterIdRecord {
    pub canister_id: Principal,
}

#[derive(CandidType, Debug, Clone, Deserialize)]
pub struct CreateCanisterSettings {
    pub controllers: Option<Vec<Principal>>,
    pub compute_allocation: Option<Nat>,
    pub memory_allocation: Option<Nat>,
    pub freezing_threshold: Option<Nat>,
}

#[derive(CandidType, Deserialize, Serialize)]
struct CanisterInstallSendArgs {
    greet: String,
    controllers: Vec<Principal>,
}

#[derive(CandidType, Debug, Clone, Deserialize)]
pub struct CreateCanisterArgs {
    pub cycles: u64,
    pub settings: CreateCanisterSettings,
}

#[derive(CandidType, Deserialize)]
struct CanisterInstall {
    mode: InstallMode,
    canister_id: Principal,
    #[serde(with = "serde_bytes")]
    wasm_module: Vec<u8>,
    #[serde(with = "serde_bytes")]
    arg: Vec<u8>,
}

const USER_DEFAULT_CYCLES: u64 = 10_000_000_000_000;
const USER_WASM: &[u8] = std::include_bytes!("../../../.dfx/local/canisters/users/users.wasm");

pub async fn call_canister_install(
    canister_id: &Principal,
    canister_install_args: Vec<u8>,
    mode: InstallMode,
) -> bool {
    let install_config: CanisterInstall = CanisterInstall {
        mode: mode,
        canister_id: canister_id.clone(),
        wasm_module: USER_WASM.to_vec(),
        arg: canister_install_args,
    };

    let ret: CallResult<()> = ic_cdk::api::call::call(
        Principal::management_canister(),
        "install_code",
        (install_config,),
    )
    .await;

    match ret {
        Ok(_x) => true,
        Err((code, msg)) => {
            print(format!(
                "An error happened during the call_canister_install: {}: {}",
                code as u8, msg
            ));
            false
        }
    }
}

pub async fn call_canister_create(
    canister_create_args: CreateCanisterArgs,
) -> Result<Principal, String> {
    #[derive(CandidType)]
    struct In {
        settings: Option<CreateCanisterSettings>,
    }

    let in_arg = In {
        settings: Some(canister_create_args.settings),
    };

    let ret: CallResult<(CanisterIdRecord,)> = ic_cdk::api::call::call_with_payment(
        Principal::management_canister(),
        "create_canister",
        (in_arg,),
        canister_create_args.cycles,
    )
    .await;

    match ret {
        Ok(x) => Ok(x.0.canister_id),
        Err((code, msg)) => Err(format!(
            "Error: call create user canister, error {} => {}",
            code as u8, msg
        )),
    }
}

pub async fn create_user_canister(helper: Principal) -> Result<Principal, String> {
    let cid = ic_cdk::api::id();
    let create_args = CreateCanisterArgs {
        cycles: USER_DEFAULT_CYCLES,
        settings: CreateCanisterSettings {
            controllers: Some(vec![cid]),
            compute_allocation: None,
            memory_allocation: None,
            freezing_threshold: None,
        },
    };

    let canister = call_canister_create(create_args.clone()).await;

    match canister {
        Ok(canister_id) => {
            let canister_install_args = Encode!(&helper).unwrap();
            call_canister_install(&canister_id, canister_install_args, InstallMode::Install).await;
            Ok(canister_id)
        }
        Err(err) => Err(err),
    }
}
