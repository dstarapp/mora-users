use candid::{CandidType, Encode, Nat};
use crc::{Algorithm, Crc};
use ic_cdk::export::{candid, Principal};
use ic_cdk::print;
use ic_cdk::storage;
use ic_cdk_macros::*;
use serde::{Deserialize, Serialize};
use std::cell::RefCell;

const USER_WASM: &[u8] = std::include_bytes!("../../../.dfx/local/canisters/likes/likes.wasm");
const INIT_CYCLES: u64 = 2_000_000_000_000;
const SLOT_SIZE: u32 = 65536; //Hash slot size

thread_local! {
    static NODE_MAP_LISTS : RefCell<CanisterNodeMapList> = RefCell::new(CanisterNodeMapList::new());
}

#[derive(CandidType, Deserialize)]
pub struct CanisterStateArg {
    canister_id: Principal,
    is_full: bool,
}

impl CanisterNodeMapList {
    pub fn new() -> Self {
        Self {
            owner: Principal::from_slice(&[]),
            slot_list: vec![],
        }
    }
}

#[derive(Clone, CandidType, Serialize, Deserialize)]
pub struct CanisterNodeMapList {
    owner: Principal,
    slot_list: Vec<CanisterNodeMap>,
}

//Hash slot node
#[derive(Clone, CandidType, Serialize, Deserialize)]
pub struct CanisterNodeMap {
    canister_id: Principal,
    start_node: u32,
    end_node: u32,
}

#[derive(CandidType, Deserialize)]
enum InstallMode {
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

//1.Install the smart contract into canister
//2.Send this canister id to install canister
async fn install_canister(canister_id: &Principal, canister_install_args: Vec<u8>) -> bool {
    let install_config: CanisterInstall = CanisterInstall {
        mode: InstallMode::Install,
        canister_id: canister_id.clone(),
        wasm_module: USER_WASM.to_vec(),
        arg: canister_install_args,
    };

    match ic_cdk::api::call::call(
        Principal::management_canister(),
        "install_code",
        (install_config,),
    )
    .await
    {
        Ok(x) => x,
        Err((code, msg)) => {
            print(format!(
                "An error happened during the call: {}: {}",
                code as u8, msg
            ));
            return false;
        }
    };
    true
}

//Create canister
async fn create_empty_canister(canister_create_args: CreateCanisterArgs) -> Principal {
    #[derive(CandidType)]
    struct In {
        settings: Option<CreateCanisterSettings>,
    }

    let in_arg = In {
        settings: Some(canister_create_args.settings),
    };

    let (create_result,): (CanisterIdRecord,) = match ic_cdk::api::call::call_with_payment(
        Principal::management_canister(),
        "create_canister",
        (in_arg,),
        canister_create_args.cycles,
    )
    .await
    {
        Ok(x) => x,
        Err((code, msg)) => {
            print(format!(
                "An error happened during the call: {}: {}",
                code as u8, msg
            ));
            (CanisterIdRecord {
                canister_id: Principal::anonymous(),
            },)
        }
    };
    create_result.canister_id
}

fn init_canister_args() -> CreateCanisterArgs {
    let this_canister_id = ic_cdk::api::id();

    CreateCanisterArgs {
        cycles: INIT_CYCLES,
        settings: CreateCanisterSettings {
            controllers: Some(vec![this_canister_id]),
            compute_allocation: None,
            memory_allocation: None,
            freezing_threshold: None,
        },
    }
}

// #[update]
// async fn fix() -> bool {
//     let nodes = NODE_MAP_LISTS.with(|list| list.borrow().slot_list.clone());
//     for elem in nodes {
//         let send_args: CanisterNodeMap = CanisterNodeMap {
//             canister_id: elem.canister_id,
//             start_node: 0,
//             end_node: SLOT_SIZE - 1,
//         };
//         let canister_install_args = Encode!(&send_args).unwrap();

//         let install_config: CanisterInstall = CanisterInstall {
//             mode: InstallMode::Upgrade,
//             canister_id: elem.canister_id,
//             wasm_module: USER_WASM.to_vec(),
//             arg: canister_install_args,
//         };

//         match ic_cdk::api::call::call(
//             Principal::management_canister(),
//             "install_code",
//             (install_config,),
//         )
//         .await
//         {
//             Ok(x) => x,
//             Err((code, msg)) => {
//                 print(format!(
//                     "An error happened during the call: {}: {}",
//                     code as u8, msg
//                 ));
//                 return false;
//             }
//         };
//     }
//     true
// }

//1.Create canisters
//2.Map node to hash slot
//3.Saving mapping table
#[update]
#[candid::candid_method(update)]
async fn batch_create_canisters(parts: u32) -> Vec<CanisterNodeMap> {
    let caller = ic_cdk::api::caller();
    assert_eq!(NODE_MAP_LISTS.with(|list| list.borrow().slot_list.len()), 0);
    assert_eq!(
        NODE_MAP_LISTS.with(|allot| { allot.borrow().owner }),
        caller
    );

    let part_size = SLOT_SIZE / parts;

    let create_args = init_canister_args();

    let controllers_id = ic_cdk::api::id();

    for i in 0..parts {
        let create_canister_id = create_empty_canister(create_args.clone()).await;

        let start_node = i * part_size;
        let end_node = start_node + (part_size - 1);

        let send_args: CanisterNodeMap = CanisterNodeMap {
            canister_id: controllers_id,
            start_node,
            end_node,
        };
        let canister_install_args = Encode!(&send_args).unwrap();

        if install_canister(&create_canister_id, canister_install_args).await {
            NODE_MAP_LISTS.with(|slot_list| {
                let mut list = slot_list.borrow_mut();
                list.slot_list.push(CanisterNodeMap {
                    canister_id: create_canister_id,
                    start_node,
                    end_node,
                });
            });
        }
    }

    NODE_MAP_LISTS.with(|list| list.borrow().slot_list.clone())
}

//insert new canister and map id to hash slot
async fn insert_single_canister(prev_canister_id: Principal) -> Result<CanisterNodeMap, String> {
    let create_args = init_canister_args();
    let controllers_id = ic_cdk::api::id();

    let create_canister_id = create_empty_canister(create_args.clone()).await;

    let mut new_start_node: u32 = 0;
    let mut new_end_node: u32 = 0;
    let mut mid_node: u32 = 0;

    NODE_MAP_LISTS.with(|slot_list| {
        let list = slot_list.borrow();
        let vecter_iterator = list.slot_list.iter();
        for elem in vecter_iterator {
            if elem.canister_id == prev_canister_id {
                mid_node = elem.start_node.clone()
                    + ((elem.end_node.clone() + 1 - elem.start_node.clone()) >> 1);

                new_start_node = mid_node.clone() + 1;
                new_end_node = elem.end_node;
            }
        }
    });

    let canister_install_args = Encode!(&CanisterNodeMap {
        canister_id: controllers_id,
        start_node: new_start_node,
        end_node: new_end_node
    })
    .unwrap();

    if install_canister(&create_canister_id, canister_install_args).await {
        NODE_MAP_LISTS.with(|list_ref| {
            let mut list = list_ref.borrow_mut();

            //updata node
            for elem in list.slot_list.iter_mut() {
                if elem.canister_id == prev_canister_id {
                    elem.end_node = mid_node;
                }
            }

            list.slot_list.push(CanisterNodeMap {
                canister_id: create_canister_id.clone(),
                start_node: new_start_node.clone(),
                end_node: new_end_node.clone(),
            });
        });

        Ok(CanisterNodeMap {
            canister_id: create_canister_id.clone(),
            start_node: new_start_node.clone(),
            end_node: new_end_node.clone(),
        })
    } else {
        Err("Expand memory false!".to_string())
    }
}

//Get the currently owned User Canister and Hash slot mapping table
#[query]
#[candid::candid_method(query)]
fn allot_canister_list() -> Vec<CanisterNodeMap> {
    NODE_MAP_LISTS.with(|list| list.borrow().slot_list.clone())
}

// Get the Canister ID of the node
#[query]
#[candid::candid_method(query)]
fn get_correlation_canister(key: String) -> Principal {
    let mod_size = crc16(&key) % (SLOT_SIZE - 1);

    NODE_MAP_LISTS.with(|list| {
        let list = list.borrow();
        let vecter_iterator = list.slot_list.iter();

        let mut canister_id: Principal = Principal::from_slice(&[]);

        for elem in vecter_iterator {
            if mod_size >= elem.start_node && mod_size <= elem.end_node {
                canister_id = elem.canister_id.clone();
            }
        }
        canister_id
    })
}

#[query]
#[candid::candid_method(query)]
fn verify_canister(pid: Principal) -> bool {
    is_exisr(pid)
}

#[update]
#[candid::candid_method(update)]
async fn hset(key: String, field: String, value: Vec<u8>) {
    let target_id = get_correlation_canister(key.clone());
    match ic_cdk::api::call::call(target_id, "hset", (key, field, value)).await {
        Ok(x) => x,
        Err((code, msg)) => {
            ic_cdk::api::print(format!(
                "An error happened during the call: {}: {}",
                code as u8, msg
            ));
        }
    }
}

#[update]
#[candid::candid_method(update)]
async fn hdel(key: String, field: String) {
    let target_id = get_correlation_canister(key.clone());
    match ic_cdk::api::call::call(target_id, "hdel", (key, field)).await {
        Ok(x) => x,
        Err((code, msg)) => {
            ic_cdk::api::print(format!(
                "An error happened during the call: {}: {}",
                code as u8, msg
            ));
        }
    }
}

#[query]
#[candid::candid_method(query)]
async fn hscan(key: String, _field: String) -> Principal {
    get_correlation_canister(key)
}

fn crc16(pid: &String) -> u32 {
    const CUSTOM_ALG: Algorithm<u16> = Algorithm {
        width: 16,
        poly: 0x8005,
        init: 0xffff,
        refin: false,
        refout: false,
        xorout: 0x0000,
        check: 0xaee7,
        residue: 0x0000,
    };
    let crc = Crc::<u16>::new(&CUSTOM_ALG);
    let mut digest = crc.digest();

    digest.update(pid.as_bytes());

    digest.finalize() as u32
}

//1.Response capacity expansion message
//2.insert new canister,update slot node
//3.send slot node message
#[update]
async fn expand_memory() {
    let caller_id = ic_cdk::api::caller();
    assert_eq!(true, is_exisr(caller_id));

    let call_arg = ic_cdk::api::call::arg_data::<(Option<CanisterStateArg>,)>().0;
    match call_arg {
        Some(call_value) => {
            let prev_canister_id = call_value.canister_id;
            if call_value.is_full == true {
                let result = insert_single_canister(prev_canister_id.clone()).await;

                match result {
                    Ok(v) => callback_new_canister_args(prev_canister_id, v).await,
                    Err(e) => ic_cdk::api::print(format!("{}", e)),
                }
            }
        }
        None => ic_cdk::api::print("Get capacity expansion information empty!"),
    };
}

async fn callback_new_canister_args(to_canister_id: Principal, new_canister_args: CanisterNodeMap) {
    match ic_cdk::call(to_canister_id, "update_node_data", (new_canister_args,)).await {
        Ok(x) => x,
        Err((code, msg)) => {
            ic_cdk::api::print(format!(
                "An error happened during the call: {}: {}",
                code as u8, msg
            ));
        }
    };
}

#[pre_upgrade]
fn pre_upgrade() {
    NODE_MAP_LISTS.with(|allot_list| {
        storage::stable_save((allot_list,)).unwrap();
    });
}

#[post_upgrade]
fn post_upgrade() {
    let (old_state,): (CanisterNodeMapList,) = storage::stable_restore().unwrap();
    NODE_MAP_LISTS.with(|allot_ids| {
        *allot_ids.borrow_mut() = old_state;
    })
}

#[init]
fn init() {
    NODE_MAP_LISTS.with(|owner_ref| {
        let mut owner = owner_ref.borrow_mut();
        owner.owner = ic_cdk::api::caller();
    })
}

#[query]
#[candid::candid_method(query)]
fn wallet_balance() -> u64 {
    ic_cdk::api::canister_balance()
}

#[update]
#[candid::candid_method(update)]
async fn wallet_receive() -> () {
    let available = ic_cdk::api::call::msg_cycles_available128();
    if available > 0 {
        ic_cdk::api::call::msg_cycles_accept128(available);
    };
}

pub fn is_exisr(caller_id: Principal) -> bool {
    let mut exist = false;
    NODE_MAP_LISTS.with(|lists_ref| {
        let lists = lists_ref.borrow();
        for elem in lists.slot_list.iter() {
            if elem.canister_id == caller_id {
                exist = true;
            }
        }
    });
    exist
}

candid::export_service!();
#[ic_cdk_macros::query(name = "__get_candid_interface_tmp_hack")]
pub fn export_candid() -> String {
    __export_service()
}
