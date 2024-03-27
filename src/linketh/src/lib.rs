use candid::{candid_method, CandidType};
use easy_hasher::easy_hasher::*;
use ic_cdk::export::{candid, Principal};
use ic_cdk::println;
use ic_cdk::*;
use ic_cdk_macros::*;
use libsecp256k1::recover;
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::convert::TryInto;

thread_local! {
    static STATE : RefCell<AllState> = RefCell::new(AllState::default());
}

#[derive(CandidType, Clone, Deserialize)]
pub struct AllState {
    owner: Option<Principal>,
    stores: BTreeMap<Principal, Profile>,
}

impl Default for AllState {
    fn default() -> Self {
        AllState {
            owner: None,
            stores: BTreeMap::new(),
        }
    }
}

#[derive(Clone, Debug, CandidType, Deserialize, Serialize)]
struct Profile {
    pub address: String,
    pub created: u64,
}

impl Default for Profile {
    fn default() -> Self {
        Profile {
            address: String::from(""),
            created: api::time(),
        }
    }
}

#[query(name = "getProfileByPrincipal")]
#[candid_method(query, rename = "getProfileByPrincipal")]
fn get_by_principal(principal: Principal) -> Option<Profile> {
    let profile = STATE.with(|s| {
        let state = s.borrow();
        match state.stores.get(&principal) {
            Some(x) => Some(x.clone()),
            _ => None,
        }
    });
    return profile;
}

#[query(name = "getByEth")]
#[candid_method(query, rename = "getByEth")]
fn get_by_eth(eth_address: String) -> Option<Profile> {
    STATE.with(|s| {
        let state = s.borrow();

        for (_, profile) in state.stores.iter() {
            if profile
                .address
                .to_lowercase()
                .eq(&eth_address.to_lowercase())
            {
                return Some(profile.clone());
            }
        }
        None
    })
}

#[query(name = "whoami")]
#[candid_method(query)]
fn whoami() -> Principal {
    ic_cdk::caller()
}

// #[query]
// #[candid_method(query)]
// fn list() -> Vec<Profile> {
//     STATE.with(|s| {
//         let state = s.borrow();
//         let mut profiles: Vec<Profile> = Vec::new();
//         for (_, profile) in state.stores.iter() {
//             profiles.push(profile.clone());
//         }
//         profiles
//     })
// }

#[query(name = "getPrincipalByEth")]
#[candid_method(query, rename = "getPrincipalByEth")]
fn get_principal_by_eth(eth_address: String) -> Option<Principal> {
    STATE.with(|s| {
        let state = s.borrow();
        for (principal, profile) in state.stores.iter() {
            if profile
                .address
                .to_lowercase()
                .eq(&eth_address.to_lowercase())
            {
                return Some(principal.clone());
            }
        }
        None
    })
}

#[update(name = "linkAddress")]
#[candid_method(update, rename = "linkAddress")]
fn link_address(message: String, signature: String) -> bool {
    let mut signature_bytes = hex::decode(signature.trim_start_matches("0x")).unwrap();
    let recovery_byte = signature_bytes.pop().expect("No recovery byte");
    let recovery_id = libsecp256k1::RecoveryId::parse_rpc(recovery_byte).unwrap();
    let signature_slice = signature_bytes.as_slice();
    let signature_bytes: [u8; 64] = signature_slice.try_into().unwrap();
    let signature = libsecp256k1::Signature::parse_standard(&signature_bytes).unwrap();
    let message_bytes = hex::decode(message.trim_start_matches("0x")).unwrap();
    let message_bytes: [u8; 32] = message_bytes.try_into().unwrap();
    let message = libsecp256k1::Message::parse(&message_bytes);
    let key = recover(&message, &signature, &recovery_id).unwrap();
    let key_bytes = key.serialize();
    let keccak256 = raw_keccak256(key_bytes[1..].to_vec());
    let keccak256_hex = keccak256.to_hex_string();
    let mut address: String = "0x".to_owned();
    address.push_str(&keccak256_hex[24..]);

    println!("Linked eth address {:?}", address);

    STATE.with(|s| {
        let mut state = s.borrow_mut();
        match state.stores.get(&api::caller()) {
            Some(_) => false,
            _ => {
                let mut profile = Profile::default();
                profile.address = address;
                state.stores.insert(api::caller(), profile);
                true
            }
        }
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

#[pre_upgrade]
fn pre_upgrade() {
    let state = STATE.with(|s| s.replace(AllState::default()));
    storage::stable_save((state,)).unwrap();
}

#[post_upgrade]
fn post_upgrade() {
    let (old_state,): (AllState,) = storage::stable_restore().unwrap();
    STATE.with(|s| {
        s.replace(old_state);
    })
}

#[init]
fn init() {
    STATE.with(|s| {
        let mut state = s.borrow_mut();
        state.owner = Some(ic_cdk::api::caller());
    })
}

candid::export_service!();
#[query]
#[candid_method(query)]
pub fn __get_candid_interface_tmp_hack() -> String {
    __export_service()
}
