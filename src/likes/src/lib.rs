use candid::CandidType;
use crc::{Algorithm, Crc};
use ic_cdk::export::{candid, Principal};
use ic_cdk::storage;
use ic_cdk_macros::*;
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::mem;

// #[path = "hashset.rs"]
mod hashset;
use hashset::HashSet;

const SLOT_SIZE: u32 = 65536; //Hash slot size
const INDEX_LIMIT: usize = 1000;
const MEMORY_LIMIT: u64 = 2 * 1024 * 1024 * 1024;

#[derive(CandidType, Deserialize)]
enum InstallMode {
    #[serde(rename = "install")]
    Install,
    #[serde(rename = "reinstall")]
    Reinstall,
    #[serde(rename = "upgrade")]
    Upgrade,
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

#[derive(Clone, CandidType, Serialize, Deserialize)]
pub struct CanisterNodeMap {
    canister_id: Principal,
    start_node: u32,
    end_node: u32,
}

impl CanisterNodeMap {
    fn new() -> Self {
        Self {
            canister_id: Principal::from_slice(&[]),
            start_node: 0,
            end_node: 0,
        }
    }
}

#[derive(Clone, CandidType, Serialize, Deserialize)]
pub struct CanisterState {
    likes: HashSet<String, Vec<u8>>,
    init_args: CanisterNodeMap,
    migrating_data: bool,
    owner: Principal,
}
impl CanisterState {
    fn new() -> Self {
        Self {
            likes: HashSet::new(),
            init_args: CanisterNodeMap::new(),
            migrating_data: false,
            owner: Principal::from_slice(&[]),
        }
    }
}

#[derive(CandidType, Deserialize)]
pub struct CanisterStateArg {
    canister_id: Principal,
    is_full: bool,
}

thread_local! {
    pub static STATE : RefCell<CanisterState> = RefCell::new(CanisterState::new());
}

#[update]
#[candid::candid_method(update)]
async fn hset(key: String, field: String, value: Vec<u8>) -> bool {
    assert_eq!(STATE.with(|state| state.borrow().migrating_data), false);

    assert_eq!(
        STATE.with(|state| state.borrow().owner),
        ic_cdk::api::caller()
    );

    let mut data_state = false;

    STATE.with(|state_ref| {
        let mut state = state_ref.borrow_mut();
        data_state = state.likes.insert(key, field, value);
    });

    read_memory_limit();
    data_state
}

#[update]
#[candid::candid_method(update)]
pub fn hdel(key: String, field: String) -> bool {
    assert_eq!(STATE.with(|state| state.borrow().migrating_data), false);

    assert_eq!(
        STATE.with(|state| state.borrow().owner),
        ic_cdk::api::caller()
    );

    STATE.with(|state_ref| {
        let mut state = state_ref.borrow_mut();
        state.likes.remove_field(key, field)
    })
}

#[query]
#[candid::candid_method(query)]
pub fn hget(key: String, field: String) -> Option<Vec<u8>> {
    STATE.with(|state_ref| {
        let state = state_ref.borrow();
        state.likes.get_field(&key, &field)
    })
}

#[query]
#[candid::candid_method(query)]
pub fn hexist(key: String, field: String) -> bool {
    STATE.with(|state_ref| {
        let state = state_ref.borrow();
        let ex = state.likes.get_field(&key, &field);
        match ex {
            Some(_v) => true,
            None => false,
        }
    })
}

//Registered user data collected,read canister memory capacity,triggered "Full" message
fn read_memory_limit() {
    STATE.with(|state| {
        let index = state.borrow().likes.len();

        //Check memory capacity after storing a batch of users
        if (index % INDEX_LIMIT) == 0 {
            //Capacity greater than 2 GB,triggered "Full" message
            if read_stable_memory_size() > MEMORY_LIMIT {
                send_expand_memory();
            }
        }
    });
}

//Send users allot canister capacity expansion information
fn send_expand_memory() {
    ic_cdk::spawn(async {
        let allot_id = STATE.with(|state| state.borrow().init_args.canister_id);
        let send_arg: CanisterStateArg = CanisterStateArg {
            canister_id: ic_cdk::api::id(),
            is_full: true,
        };
        match ic_cdk::call(allot_id.clone(), "expand_memory", (send_arg,)).await {
            Ok(x) => x,
            Err((code, msg)) => {
                ic_cdk::api::print(format!(
                    "An error happened during the call: {}: {}",
                    code as u8, msg
                ));
            }
        }
    });
}

//1.Get users allot canister update node and data information
//2.Split user data by new node
//3.Send user data to new inserted canister
#[update]
async fn update_node_data() {
    assert_eq!(ic_cdk::api::caller(), get_allot_id());

    let call_arg = ic_cdk::api::call::arg_data::<(Option<CanisterNodeMap>,)>().0;

    match call_arg {
        Some(call_value) => {
            let move_data = division_migration_data(call_value.start_node, call_value.end_node);
            sand_migration_data(call_value.canister_id, move_data).await;
        }
        None => ic_cdk::api::print("Update node call error!"),
    };
}

//Move user data of corresponding node to new canister
async fn sand_migration_data(to_canister_id: Principal, move_data: HashSet<String, Vec<u8>>) {
    for (k, v) in move_data.hset {
        let mut data: HashSet<String, Vec<u8>> = HashSet::new();
        data.hset.insert(k, v);
        match ic_cdk::call(to_canister_id, "receive_migration_data", (data,)).await {
            Ok(x) => x,
            Err((code, msg)) => {
                ic_cdk::api::print(format!(
                    "An error happened during the call: {}: {}",
                    code as u8, msg
                ));
            }
        };
    }
}

#[update]
async fn receive_migration_data(move_data: HashSet<String, Vec<u8>>) {
    let caller = ic_cdk::api::caller();
    let allot_id = get_allot_id();

    match ic_cdk::call(allot_id, "verify_canister", (caller,)).await {
        Ok(x) => x,
        Err((code, msg)) => {
            ic_cdk::api::print(format!(
                "An error happened during the call verify_canister: {}: {}",
                code as u8, msg
            ));
            return;
        }
    }
    STATE.with(|state| state.borrow_mut().migrating_data = true);
    put_data(move_data);
    STATE.with(|state| state.borrow_mut().migrating_data = false);
}

fn put_data(likes: HashSet<String, Vec<u8>>) {
    STATE.with(|state_ref| {
        let mut state = state_ref.borrow_mut();
        for (k, v) in likes.hset {
            state.likes.hset.insert(k, v);
        }
    });
}

//Get node interval value and move data
fn division_migration_data(start_node: u32, end_node: u32) -> HashSet<String, Vec<u8>> {
    STATE.with(|state| state.borrow_mut().migrating_data = true);

    STATE.with(|state_ref| {
        let mut original_data = state_ref.borrow_mut();

        let mut move_data: HashSet<String, Vec<u8>> = HashSet::new();
        for (k, v) in original_data.likes.hset.iter() {
            let mod_size = crc16(&k) % (SLOT_SIZE - 1);
            if mod_size >= start_node && mod_size <= end_node {
                move_data.hset.insert(k.clone(), v.clone());
            }
        }

        original_data.likes.hset.retain(|k, _| {
            (crc16(&k.to_string()) % (SLOT_SIZE - 1)) < start_node
                || (crc16(&k.to_string()) % (SLOT_SIZE - 1)) > end_node
        });

        STATE.with(|state| state.borrow_mut().migrating_data = false);
        move_data
    })
}

//canister stable memory
#[query(name = "memory_size")]
pub fn read_stable_memory_size() -> u64 {
    #[cfg(target_arch = "wasm32")]
    {
        users_memory_size() as u64
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        (ic_cdk::api::stable::stable_size() as u64) * 65536
    }
}

//canister stable memory
#[query(name = "st_memory_size")]
pub fn stable_memory_size() -> u64 {
    (ic_cdk::api::stable::stable_size() as u64) * 65536
}

pub fn users_memory_size() -> usize {
    STATE.with(|state_ref| {
        let state = state_ref.borrow();
        hset_size(&state.likes)
    })
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

#[pre_upgrade]
fn pre_upgrade() {
    STATE.with(|state| {
        storage::stable_save((state,)).unwrap();
    })
}

#[post_upgrade]
fn post_upgrade() {
    let (old_state,): (CanisterState,) = storage::stable_restore().unwrap();
    STATE.with(|state| {
        *state.borrow_mut() = old_state;
    })
}

#[init]
fn init() {
    let init = ic_cdk::api::call::arg_data::<(Option<CanisterNodeMap>,)>().0;
    match init {
        Some(init_args) => STATE.with(|state_ref| {
            let mut state = state_ref.borrow_mut();
            state.init_args = init_args;
            state.owner = ic_cdk::api::caller();
        }),
        None => ic_cdk::api::print("Get caninster init args error!"),
    }
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

pub fn hset_size(hash_set: &HashSet<String, Vec<u8>>) -> usize {
    let mut size = 0;
    for (k, v) in &hash_set.hset {
        let mut f_k_s: usize = mem::size_of::<String>() + mem::size_of_val(&**k);
        let mut f_v_s: usize = mem::size_of_val(v);
        for (f_k, f_v) in &v.field {
            f_k_s += std::mem::size_of::<String>() + std::mem::size_of_val(&**f_k);
            f_v_s += std::mem::size_of_val(f_v) + std::mem::size_of_val(&**f_v);
        }
        size += f_k_s + f_v_s;
    }
    size
}

fn get_allot_id() -> Principal {
    STATE.with(|state_ref| {
        let state = state_ref.borrow();
        state.init_args.canister_id
    })
}

candid::export_service!();
// // #[ic_cdk_macros::query(name = "__get_candid_interface_tmp_hack")]
pub fn export_candid() -> String {
    __export_service()
}
