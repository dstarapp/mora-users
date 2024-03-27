use candid::{candid_method, CandidType, Encode};
use ic_cdk::export::{candid, Principal};
use ic_cdk::print;
use ic_cdk_macros::*;
use serde::Deserialize;

mod dao;
mod install;
mod state;
mod user;

use dao::MoraDaoService;
use install::*;
use state::*;
use user::{PlanetMsg, UserInfo, UserService};

#[derive(CandidType, Deserialize)]
struct UserLoginResp {
    canister_id: Principal,
    userinfo: UserInfo,
}

#[update]
async fn update_canister(canister: Principal) -> bool {
    assert!(get_sim_owner() == ic_cdk::api::caller());
    let helper = get_sim_helper();

    let canister_install_args = Encode!(&helper).unwrap();
    return call_canister_install(&canister, canister_install_args, InstallMode::Upgrade).await;
}

#[query]
#[candid_method(query)]
fn total_count() -> u64 {
    get_user_count()
}

#[query]
#[candid_method(query)]
fn canister_count() -> u64 {
    get_canister_count()
}

#[query]
#[candid_method(query)]
fn search_canister(user: Principal) -> Option<Principal> {
    return get_user_canister(user);
}

#[query]
#[candid_method(query)]
fn search_index(user: Principal) -> u128 {
    return get_user_index(user);
}

#[query(name = "canister_list")]
#[candid_method(query)]
fn canister_list() -> Vec<Principal> {
    return get_canister_list();
}

#[query(name = "get_canister")]
#[candid_method(query)]
fn get_canister() -> Option<Principal> {
    let caller = ic_cdk::api::caller();
    return get_user_canister(caller);
}

#[query(name = "verify_canister")]
#[candid_method(query)]
fn verify_canister(canister: Principal) -> bool {
    has_canister(canister)
}

#[update(name = "login")]
#[candid_method(update)]
async fn login() -> Result<UserLoginResp, String> {
    let caller = ic_cdk::api::caller();
    assert_ne!(caller, Principal::anonymous());

    login_call(caller).await
}

#[update(name = "login_test")]
#[candid_method(update)]
async fn login_test(user: Principal) -> Result<UserLoginResp, String> {
    let caller = ic_cdk::api::caller();
    assert_eq!(caller, get_sim_owner());
    login_call(user).await
}

#[query]
#[candid::candid_method(query)]
fn wallet_balance() -> u64 {
    ic_cdk::api::canister_balance()
}

#[update]
#[candid::candid_method(update)]
fn wallet_receive() -> () {
    let available = ic_cdk::api::call::msg_cycles_available128();
    if available > 0 {
        ic_cdk::api::call::msg_cycles_accept128(available);
    };
}

#[query]
fn get_helper() -> Principal {
    get_sim_helper()
}

#[update]
async fn notify_planet_msg(msg: PlanetMsg) -> bool {
    let pid = ic_cdk::api::caller();
    // verify pid is planet ( call hepler verify)
    let dao = MoraDaoService(get_helper());
    match dao.verify_planet(pid).await {
        Ok((valid,)) => {
            if !valid {
                return false;
            }
        }
        Err((code, msg)) => {
            print(format!(
                "An error happened during verifyPlanet: {}: {}",
                code as u8, msg
            ));
            return false;
        }
    }

    let canister_id = get_user_canister(msg.user);
    if canister_id.is_none() {
        return false;
    };

    let service = UserService(canister_id.expect("unkown user canister id"));
    match service.on_planet_msg(pid, msg).await {
        Ok((ok,)) => {
            return ok;
        }
        Err((code, msg)) => {
            print(format!(
                "An error happened during on_planet_msg: {}: {}",
                code as u8, msg
            ));
            return false;
        }
    };
}

#[pre_upgrade]
fn pre_upgrade() {
    state_save();
}

#[post_upgrade]
fn post_upgrade() {
    state_restore();
}

#[init]
#[candid_method(init)]
fn init(helper: Principal) {
    print(format!("helper id: {}", helper));
    state_set(ic_cdk::api::caller(), Some(helper));
}

async fn login_call(caller: Principal) -> Result<UserLoginResp, String> {
    let canister_id = match get_user_canister(caller) {
        Some(canister) => canister,
        _ => {
            let canister = register_user(caller).await;
            match canister {
                Ok(cid) => cid,
                Err(err) => return Err(err),
            }
        }
    };
    let service = UserService(canister_id);
    match service.login_proxy(caller).await {
        Ok((userinfo,)) => {
            return Ok(UserLoginResp {
                canister_id: canister_id,
                userinfo: userinfo,
            })
        }
        Err((code, msg)) => {
            return Err(format!(
                "login error during the call: {}: {}",
                code as u8, msg
            ));
        }
    };
}

candid::export_service!();
#[query]
#[candid_method(query)]
pub fn __get_candid_interface_tmp_hack() -> String {
    __export_service()
}

#[cfg(not(any(target_arch = "wasm32", test)))]
fn main() {
    std::print!("{}", __export_service());
}

#[cfg(any(target_arch = "wasm32", test))]
fn main() {}
