// This file is part of Rundler.
//
// Rundler is free software: you can redistribute it and/or modify it under the
// terms of the GNU Lesser General Public License as published by the Free Software
// Foundation, either version 3 of the License, or (at your option) any later version.
//
// Rundler is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
// See the GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License along with Rundler.
// If not, see https://www.gnu.org/licenses/.

//! Chain specification for Rundler

use std::str::FromStr;

use ethers::types::{Address, U256};
use serde::{Deserialize, Serialize};

const ENTRY_POINT_ADDRESS_V6_0: &str = "0x5FF137D4b0FDCD49DcA30c7CF57E578a026d2789";

/// Chain specification for Rundler
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ChainSpec {
    /*
     * Chain constants
     */
    /// name for logging purposes, e.g. "Ethereum", no logic is performed on this
    pub name: String,
    /// chain id
    pub id: u64,
    /// entry point address
    pub entry_point_address: Address,

    /*
     * Gas estimation
     */
    /// true if calldata is priced in preVerificationGas
    pub calldata_pre_verification_gas: bool,
    /// type of gas oracle contract for pricing calldata in preVerificationGas
    /// If calldata_pre_verification_gas is true, this must not be None
    pub l1_gas_oracle_contract_type: L1GasOracleContractType,
    /// address of gas oracle contract for pricing calldata in preVerificationGas
    pub l1_gas_oracle_contract_address: Address,
    /// true if L1 calldata gas should be included in the gas limit
    /// only applies when calldata_pre_verification_gas is true
    pub include_l1_gas_in_gas_limit: bool,

    /*
     * Fee estimation
     */
    /// true if eip1559 is enabled, and thus priority fees are used
    pub eip1559_enabled: bool,
    /// Type of oracle for estimating priority fees
    pub priority_fee_oracle_type: PriorityFeeOracleType,
    /// Minimum max priority fee per gas for the network
    pub min_max_priority_fee_per_gas: U256,
    /// Maximum max priority fee per gas for the network
    pub max_max_priority_fee_per_gas: U256,

    /*
     * Bundle building
     */
    /// The maximum amount of time to wait before sending a bundle.
    ///
    /// The bundle builder will always try to send a bundle when a new block is received.
    /// This parameter is used to trigger the builder to send a bundle after a specified
    /// amount of time, before a new block is not received.
    pub bundle_max_send_interval_millis: u64,

    /*
     * Senders
     */
    /// True if the flashbots sender is enabled on this chain
    pub flashbots_enabled: bool,
    /// True if the bloxroute sender is enabled on this chain
    pub bloxroute_enabled: bool,

    /*
     * Pool
     */
    /// Size of the chain history to keep to handle reorgs
    pub chain_history_size: u64,
}

/// Type of gas oracle contract for pricing calldata in preVerificationGas
#[derive(Clone, Copy, Debug, Deserialize, Default, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum L1GasOracleContractType {
    /// No gas oracle contract
    #[default]
    None,
    /// Arbitrum Nitro type gas oracle contract
    ArbitrumNitro,
    /// Optimism Bedrock type gas oracle contract
    OptimismBedrock,
}

/// Type of oracle for estimating priority fees
#[derive(Clone, Debug, Deserialize, Default, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PriorityFeeOracleType {
    /// Use eth_maxPriorityFeePerGas on the provider
    #[default]
    Provider,
    /// Use the usage based oracle
    UsageBased,
}

impl Default for ChainSpec {
    fn default() -> Self {
        Self {
            name: "Unknown".to_string(),
            id: 0,
            entry_point_address: Address::from_str(ENTRY_POINT_ADDRESS_V6_0).unwrap(),
            eip1559_enabled: true,
            calldata_pre_verification_gas: false,
            l1_gas_oracle_contract_type: L1GasOracleContractType::default(),
            l1_gas_oracle_contract_address: Address::zero(),
            include_l1_gas_in_gas_limit: true,
            priority_fee_oracle_type: PriorityFeeOracleType::default(),
            min_max_priority_fee_per_gas: U256::zero(),
            max_max_priority_fee_per_gas: U256::MAX,
            bundle_max_send_interval_millis: u64::MAX,
            flashbots_enabled: false,
            bloxroute_enabled: false,
            chain_history_size: 64,
        }
    }
}
