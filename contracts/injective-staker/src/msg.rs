use crate::state::{Allocation, UserStatus, ValidatorInfo};
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Binary, Uint128, Uint256};

#[cw_serde]
pub struct InstantiateMsg {
    pub treasury: Addr,
    pub default_validator: Addr,
}

#[cw_serde]
pub enum ExecuteMsg {
    SetFee {
        new_fee: u16,
    },
    SetDistributionFee {
        new_distribution_fee: u16,
    },
    SetMinimumDeposit {
        new_min_deposit: Uint128,
    },
    SetTreasury {
        new_treasury_addr: Addr,
    },
    SetDefaultValidator {
        new_default_validator_addr: Addr,
    },
    /// Transfer is a base message to move tokens to another account without triggering actions
    Transfer {
        recipient: String,
        amount: Uint128,
    },
    /// Send is a base message to transfer tokens to a contract and trigger an action
    /// on the receiving contract. Receiving contract must implement receiver interface.
    Send {
        contract: String,
        amount: Uint128,
        msg: Binary,
    },
    Stake {},
    StakeToSpecificValidator {
        validator_addr: Addr,
    },
    Unstake {
        amount: Uint128,
    },
    UnstakeFromSpecificValidator {
        validator_addr: Addr,
        amount: Uint128,
    },
    Claim {},
    AddValidator {
        validator: Addr,
    },
    EnableValidator {
        validator: Addr,
    },
    DisableValidator {
        validator: Addr,
    },
    // Whitelist messages
    AddAgent {
        agent: Addr,
    },
    RemoveAgent {
        agent: Addr,
    },
    SetPendingOwner {
        new_owner: Addr,
    },
    ClaimOwnership {},
    AddUserToWhitelist {
        user: Addr,
    },
    AddUserToBlacklist {
        user: Addr,
    },
    ClearUserStatus {
        user: Addr,
    },
    Pause,
    Unpause,
    CompoundRewards,
    Allocate {
        recipient: Addr,
        amount: Uint128,
    },
    Deallocate {
        recipient: Addr,
        amount: Uint128,
    },
    DistributeRewards {
        recipient: Addr,
        in_inj: bool,
    },
    // Internal messages
    Restake {
        amount: Uint128,
        validator_addr: Addr,
    },
    // Test messages
    #[cfg(any(test, feature = "test"))]
    TestAllocate {
        recipient: Addr,
        amount: Uint128,
    },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(GetStakerInfoResponse)]
    GetStakerInfo {},
    #[returns(cw20::BalanceResponse)]
    Balance { address: String },
    #[returns(cw20::TokenInfoResponse)]
    TokenInfo {},
    #[returns(cw20::MarketingInfoResponse)]
    MarketingInfo {},
    #[returns(GetIsOwnerResponse)]
    IsOwner { addr: Addr },
    #[returns(GetValidatorResponse)]
    GetValidators {},
    #[returns(GetTotalStakedResponse)]
    GetTotalStaked {},
    #[returns(GetTotalRewardsResponse)]
    GetTotalRewards {},
    #[returns(GetTotalSupplyResponse)]
    GetTotalSupply {},
    #[returns(GetClaimableAmountResponse)]
    GetClaimableAmount { user: Addr },

    // Whitelist queries
    #[returns(GetIsAgentResponse)]
    IsAgent { agent: Addr },
    #[returns(GetIsWhitelistedResponse)]
    IsWhitelisted { user: Addr },
    #[returns(GetIsBlacklistedResponse)]
    IsBlacklisted { user: Addr },
    #[returns(GetCurrentUserStatusResponse)]
    GetCurrentUserStatus { user: Addr },
    #[returns(GetSharePriceResponse)]
    GetSharePrice {},
    #[returns(GetTotalAssetsResponse)]
    GetTotalAssets {},
    #[returns(cw_controllers::ClaimsResponse)]
    GetClaimableAssets { user: Addr },
    #[returns(GetMaxWithdrawResponse)]
    GetMaxWithdraw { user: Addr },
    #[returns(GetAllocationsResponse)]
    GetAllocations { user: Addr },
    #[returns(GetTotalAllocatedResponse)]
    GetTotalAllocated { user: Addr },
    #[returns(GetDistributionAmountsResponse)]
    GetDistributionAmounts {
        distributor: Addr,
        recipient: Option<Addr>,
    },
}

#[cw_serde]
pub struct GetStakerInfoResponse {
    pub owner: Addr,
    pub default_validator: Addr,
    pub treasury: Addr,
    pub fee: u16,
    pub distribution_fee: u16,
    pub min_deposit: Uint128,
    pub is_paused: bool,
}

#[cw_serde]
pub struct GetValidatorResponse {
    pub validators: Vec<ValidatorInfo>,
}

#[cw_serde]
pub struct GetIsAgentResponse {
    pub is_agent: bool,
}

#[cw_serde]
pub struct GetIsOwnerResponse {
    pub is_owner: bool,
}

#[cw_serde]
pub struct GetIsWhitelistedResponse {
    pub is_whitelisted: bool,
}

#[cw_serde]
pub struct GetIsBlacklistedResponse {
    pub is_blacklisted: bool,
}

#[cw_serde]
pub struct GetCurrentUserStatusResponse {
    pub user_status: UserStatus,
}

#[cw_serde]
pub struct GetTotalStakedResponse {
    pub total_staked: Uint128,
}

#[cw_serde]
pub struct GetTotalRewardsResponse {
    pub total_rewards: Uint128,
}

#[cw_serde]
pub struct GetTotalSupplyResponse {
    pub total_supply: Uint128,
}

#[cw_serde]
pub struct GetSharePriceResponse {
    pub numerator: Uint256,
    pub denominator: Uint256,
}

#[cw_serde]
pub struct GetTotalAssetsResponse {
    pub total_assets: Uint128,
}

#[cw_serde]
pub struct GetMaxWithdrawResponse {
    pub max_withdraw: Uint128,
}

#[cw_serde]
pub struct GetClaimableAmountResponse {
    pub claimable_amount: Uint128,
}

#[cw_serde]
pub struct GetAllocationsResponse {
    pub allocations: Vec<Allocation>,
}

#[cw_serde]
pub struct GetTotalAllocatedResponse {
    pub total_allocated_amount: Uint128,
    pub total_allocated_share_price_num: Uint256,
    pub total_allocated_share_price_denom: Uint256,
}

#[cw_serde]
pub struct GetDistributionAmountsResponse {
    pub inj_amount: Uint128,
    pub truinj_amount: Uint128,
    pub distribution_fee: Uint128,
}
