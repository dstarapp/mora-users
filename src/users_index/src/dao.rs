// This is an experimental feature to generate Rust binding from Candid.
// You may want to manually adjust some of the types.
use ic_cdk::api::call::CallResult;
use ic_cdk::export::candid::{self};

pub struct MoraDaoService(pub candid::Principal);
impl MoraDaoService {
    pub async fn verify_planet(&self, arg0: candid::Principal) -> CallResult<(bool,)> {
        ic_cdk::call(self.0, "verifyPlanet", (arg0,)).await
    }
}
