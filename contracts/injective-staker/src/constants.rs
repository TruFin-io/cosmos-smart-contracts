pub const ONE_INJ: u128 = 1_000_000_000_000_000_000;
pub const FEE_PRECISION: u16 = 10_000;
pub const SHARE_PRICE_SCALING_FACTOR: u128 = 1_000_000_000_000_000_000;
pub const INJ: &str = "inj";
/// The required time period for unbonding operations, as specified by the network.
/// Currently set to 21 days.
pub const UNBONDING_PERIOD: cw_utils::Duration = cw_utils::Duration::Time(21 * 24 * 60 * 60);
