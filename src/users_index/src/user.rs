// This is an experimental feature to generate Rust binding from Candid.
// You may want to manually adjust some of the types

use candid::Int;
use ic_cdk::api::call::CallResult;
use ic_cdk::export::candid::{CandidType, Deserialize};
use ic_cdk::export::{candid, Principal};

#[derive(CandidType, Deserialize, Debug)]
pub enum PlanetMsgType {
    #[serde(rename = "subscribe")]
    Subscribe,
    #[serde(rename = "unsubscribe")]
    Unsubscribe,
    #[serde(rename = "add")]
    Add,
    #[serde(rename = "remove")]
    Remove,
}

#[derive(CandidType, Deserialize)]
pub struct NFT {
    token_index: String,
    canister_id: Principal,
    standard: String,
}

#[derive(CandidType, Deserialize)]
pub struct UserInfo {
    nft: Option<NFT>,
    pid: Principal,
    created: Int,
    email: String,
    avatar: String,
}

#[derive(CandidType, Deserialize, Debug)]
pub struct PlanetMsg {
    pub msg_type: PlanetMsgType,
    pub user: Principal,
    pub data: Option<Vec<u8>>,
}

pub struct UserService(pub candid::Principal);
impl UserService {
    pub async fn login_proxy(&self, arg0: candid::Principal) -> CallResult<(UserInfo,)> {
        ic_cdk::call(self.0, "login_proxy", (arg0,)).await
    }
    // pub async fn syncold(&self, arg0: candid::Principal) -> CallResult<(bool,)> {
    //     ic_cdk::call(self.0, "syncold", (arg0,)).await
    // }
    pub async fn on_planet_msg(
        &self,
        arg0: candid::Principal,
        arg1: PlanetMsg,
    ) -> CallResult<(bool,)> {
        ic_cdk::call(self.0, "on_planet_msg", (arg0, arg1)).await
    }
}
