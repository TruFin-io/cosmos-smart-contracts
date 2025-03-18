use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Uint128, Uint256};
use cw20::Expiration;
use cw_controllers::Claims;
use cw_storage_plus::{Index, IndexList, IndexedMap, Item, Map, MultiIndex};
use std::fmt;

#[cw_serde]
pub struct StakerInfo {
    pub treasury: Addr,
    pub fee: u16,
    pub min_deposit: u128,
    pub distribution_fee: u16,
}

#[cw_serde]
pub struct ValidatorInfo {
    pub total_staked: Uint128,
    pub state: ValidatorState,
    pub addr: String,
}

#[cw_serde]
pub enum ValidatorState {
    None,
    Enabled,
    Disabled,
}

#[cw_serde]
pub struct Allocation {
    pub allocator: Addr,
    pub recipient: Addr,
    pub inj_amount: Uint128,
    pub share_price_num: Uint256,
    pub share_price_denom: Uint256,
}

impl Default for Allocation {
    fn default() -> Self {
        Self {
            allocator: Addr::unchecked(""),
            recipient: Addr::unchecked(""),
            inj_amount: Uint128::zero(),
            share_price_num: Uint256::zero(),
            share_price_denom: Uint256::zero(),
        }
    }
}

pub struct AllocationIndexes<'a> {
    pub allocator: MultiIndex<'a, Addr, Allocation, (Addr, Addr)>,
}

impl<'a> IndexList<Allocation> for AllocationIndexes<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<Allocation>> + '_> {
        Box::new(std::iter::once(&self.allocator as &dyn Index<Allocation>))
    }
}

pub fn allocations<'a>() -> IndexedMap<(Addr, Addr), Allocation, AllocationIndexes<'a>> {
    let indexes = AllocationIndexes {
        allocator: MultiIndex::new(
            |_pk: &[u8], d: &Allocation| d.allocator.clone(),
            "allocations",
            "allocations__allocator",
        ),
    };
    IndexedMap::new("allocations", indexes)
}

pub const STAKER_INFO: Item<StakerInfo> = Item::new("staker_info");
pub const VALIDATORS: Map<&String, ValidatorState> = Map::new("validators");
pub const DEFAULT_VALIDATOR: Item<String> = Item::new("default_validator");
pub const WHITELIST_AGENTS: Map<&Addr, ()> = Map::new("whitelist_agents");
pub const OWNER: Item<Addr> = Item::new("owner");
pub const PENDING_OWNER: Item<Addr> = Item::new("pending_owner");
pub const WHITELIST_USERS: Map<&Addr, UserStatus> = Map::new("whitelist_users");
pub const IS_PAUSED: Item<bool> = Item::new("is_paused");
pub const CONTRACT_REWARDS: Item<Uint128> = Item::new("contract_rewards");
pub const CLAIMS: Claims = Claims::new("claims");

#[cw_serde]
pub enum UserStatus {
    NoStatus,
    Whitelisted,
    Blacklisted,
}

/// Implement Display for UserStatus
impl fmt::Display for UserStatus {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let status_str = match self {
            Self::NoStatus => "no_status",
            Self::Whitelisted => "whitelisted",
            Self::Blacklisted => "blacklisted",
        };
        write!(f, "{}", status_str)
    }
}

pub trait GetValueTrait {
    fn get_value(&self) -> u64;
}
impl GetValueTrait for Expiration {
    fn get_value(&self) -> u64 {
        match self {
            Self::AtHeight(height) => *height,
            Self::AtTime(time) => time.seconds(),
            Self::Never {} => 0,
        }
    }
}
