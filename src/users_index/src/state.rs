use candid::{CandidType, Decode, Deserialize, Encode};
use ic_cdk::export::Principal;
use ic_stable_structures::memory_manager::{MemoryId, MemoryManager, VirtualMemory};
// use ic_stable_structures::reader::Reader;
use crate::install::*;
use ic_stable_structures::writer::Writer;
use ic_stable_structures::Memory;
use ic_stable_structures::{BoundedStorable, DefaultMemoryImpl, StableBTreeMap, Storable, Vec};
use std::{borrow::Cow, cell::RefCell, vec};

type VMemory = VirtualMemory<DefaultMemoryImpl>;

const MAX_KEY_SIZE: u32 = 100;
// const MAX_VALUE_SIZE: u32 = 100;
const USER_PER_SIZE: u128 = 1000;

thread_local! {
    // The memory manager is used for simulating multiple memories. Given a `MemoryId` it can
    // return a memory that can be used by stable structures.
    static MEMORY_MANAGER: RefCell<MemoryManager<DefaultMemoryImpl>> =
        RefCell::new(MemoryManager::init(DefaultMemoryImpl::default()));

    static STATE: RefCell<State> = RefCell::new(
        State::new(),
    );
}

#[derive(CandidType, Deserialize)]
struct SimState {
    owner: Principal,
    helper: Option<Principal>,
    usercount: u128,
}

impl SimState {
    fn new() -> Self {
        Self {
            owner: Principal::anonymous(),
            helper: None,
            usercount: 0,
        }
    }
}

impl Storable for SimState {
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        Cow::Owned(Encode!(self).unwrap())
    }

    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        Decode!(&bytes, Self).unwrap()
    }
}

#[derive(Eq, PartialEq, PartialOrd, Ord, Clone)]
struct StablePrincipal(Principal);
impl Storable for StablePrincipal {
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        Cow::Owned(self.0.as_slice().to_vec())
    }

    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        Self(Principal::from_slice(&bytes.to_vec()))
    }
}

impl BoundedStorable for StablePrincipal {
    const MAX_SIZE: u32 = MAX_KEY_SIZE;
    const IS_FIXED_SIZE: bool = false;
}

struct State {
    sim_state: SimState,
    reserve_memory: VMemory,
    user_canisters: RefCell<StableBTreeMap<StablePrincipal, u128, VMemory>>,
    all_canisters: RefCell<Vec<StablePrincipal, VMemory>>,
}

impl State {
    fn new() -> Self {
        let (m0, m1, m2) = MEMORY_MANAGER.with(|m| {
            let manager = m.borrow();
            (
                manager.get(MemoryId::new(0)),
                manager.get(MemoryId::new(1)),
                manager.get(MemoryId::new(2)),
            )
        });
        let all = Vec::init(m2);
        Self {
            sim_state: SimState::new(),
            reserve_memory: m0,
            user_canisters: RefCell::new(StableBTreeMap::init(m1)),
            all_canisters: RefCell::new(all.expect("state vec memory error")),
        }
    }

    fn save(&mut self) {
        let mut w = Writer::new(&mut self.reserve_memory, 0);
        let simbyte = self.sim_state.to_bytes();

        let len = simbyte.len() as u32;
        w.write(&len.to_le_bytes()).unwrap();
        w.write(&simbyte).unwrap();
    }

    fn reload(&mut self) {
        print!("on reload");
        let mut buf: [u8; 4] = [0; 4];
        self.reserve_memory.read(0, &mut buf);
        let len = u32::from_le_bytes(buf);

        let mut data = vec![0; len as usize];
        self.reserve_memory.read(4, &mut data);

        self.sim_state = SimState::from_bytes(data.to_bytes());
    }
}

pub fn state_save() {
    STATE.with(|s| {
        let mut state = s.borrow_mut();
        state.save();
    });
}

pub fn state_restore() {
    STATE.with(|s| {
        let mut state = s.borrow_mut();
        state.reload();

        print!(
            "owner: {:?}, helper: {:?}, users: {:?}",
            state.sim_state.owner, state.sim_state.helper, state.sim_state.usercount
        );
    });
}

pub fn state_set(owner: Principal, helper: Option<Principal>) {
    STATE.with(|s| {
        let mut state = s.borrow_mut();
        state.sim_state.owner = owner;
        state.sim_state.helper = helper;
    });
}

pub fn get_sim_owner() -> Principal {
    STATE.with(|s| s.borrow().sim_state.owner)
}

pub fn get_sim_helper() -> Principal {
    STATE.with(|s| s.borrow().sim_state.helper.expect("unset helper"))
}

pub async fn register_user(user: Principal) -> Result<Principal, String> {
    if check_full() {
        let canister = create_user_canister(get_sim_helper()).await;
        match canister {
            Ok(canister_id) => {
                let ret = STATE.with(|s| {
                    let state = s.borrow_mut();
                    let all_canisters = state.all_canisters.borrow_mut();
                    all_canisters.push(&StablePrincipal(canister_id)).map(|x| x)
                });
                match ret {
                    Ok(_) => {}
                    Err(err) => return Err(format!("{:?}", err)),
                };
            }
            Err(err) => return Err(err),
        };
    };

    let canister = STATE.with(|s| {
        let mut state = s.borrow_mut();
        state.sim_state.usercount = state.sim_state.usercount + 1;

        let all_canisters = state.all_canisters.borrow_mut();
        let mut user_canisters = state.user_canisters.borrow_mut();
        user_canisters.insert(StablePrincipal(user), state.sim_state.usercount);
        all_canisters
            .get(all_canisters.len() - 1)
            .expect("can not get the last canister")
    });
    Ok(canister.0)
}

pub fn check_full() -> bool {
    STATE.with(|s| {
        let state = s.borrow();
        let all_canisters = state.all_canisters.borrow();
        let canister_len = all_canisters.len() as u128;

        state.sim_state.usercount >= USER_PER_SIZE * canister_len
    })
}

pub fn get_user_canister(user: Principal) -> Option<Principal> {
    STATE.with(|s| {
        let state = s.borrow();
        let user_canisters = state.user_canisters.borrow();
        let all_canisters = state.all_canisters.borrow();
        let ret = user_canisters.get(&StablePrincipal(user));

        match ret {
            None => None,
            Some(idx) => {
                let did = ((idx - 1) / USER_PER_SIZE) as u64;
                all_canisters.get(did).map(|x| x.0)
            }
        }
    })
}

pub fn has_canister(canister: Principal) -> bool {
    STATE.with(|s| {
        let state = s.borrow();
        let all_canisters = state.all_canisters.borrow();
        for cs in all_canisters.iter() {
            if cs.0.eq(&canister) {
                return true;
            }
        }
        return false;
    })
}

pub fn get_user_count() -> u64 {
    STATE.with(|s| {
        let state = s.borrow();
        let user_canisters = state.user_canisters.borrow();
        user_canisters.len()
    })
}

pub fn get_canister_count() -> u64 {
    STATE.with(|s| {
        let state = s.borrow();
        let all_canisters = state.all_canisters.borrow();
        all_canisters.len()
    })
}

pub fn get_canister_list() -> vec::Vec<Principal> {
    STATE.with(|s| {
        let mut data = vec::Vec::new();
        let state = s.borrow();
        let all_canisters = state.all_canisters.borrow();
        for x in all_canisters.iter() {
            data.push(x.0);
        }
        data
    })
}

pub fn get_user_index(user: Principal) -> u128 {
    STATE.with(|s| {
        let state = s.borrow();
        let user_canisters = state.user_canisters.borrow();
        let ret = user_canisters.get(&StablePrincipal(user));

        match ret {
            None => 0,
            Some(idx) => idx,
        }
    })
}
