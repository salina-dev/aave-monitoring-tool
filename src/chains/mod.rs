use ethers::prelude::*;
use std::env;

use crate::chains::ethereum::ethereum_chain::get_current_block_number_ethereum;

pub mod ethereum;

pub mod pk;

pub struct PositionData {
    pub supplied_amount: U256,
    pub borrowed_amount: U256,
}

impl PositionData {
    pub fn new() -> Self {
        Self {
            supplied_amount: U256::from(0),
            borrowed_amount: U256::from(0),
        }
    }
}

pub fn get_position_data() -> Result<PositionData, String> {
    let mut position_data = PositionData::new();
    // Read initial values from environment variables
    if let Ok(supplied_amount_str) = env::var("INITIAL_SUPPLIED_AMOUNT") {
        if let Ok(amount) = supplied_amount_str.parse::<u64>() {
            position_data.supplied_amount = U256::from(amount);
        }
    }
    if let Ok(borrowed_amount_str) = env::var("INITIAL_BORROWED_AMOUNT") {
        if let Ok(amount) = borrowed_amount_str.parse::<u64>() {
            position_data.borrowed_amount = U256::from(amount);
        }
    }
    Ok(position_data)
}

// Configuration functions to read from environment variables
pub fn get_user_address_to_track() -> String {
    env::var("AAVE_USER_ADDRESS_TO_TRACK")
        .unwrap_or_else(|_| "0xBDD3B59416Fc0263354953aeeFC51Ba3A94E134e".to_string())
}

pub fn get_pool_v3_address() -> String {
    env::var("AAVE_POOL_V3_ADDRESS")
        .unwrap_or_else(|_| "0x87870Bca3F3fD6335C3F4ce8392D69350B4fA4E2".to_string())
}

pub fn get_supply_token_address() -> String {
    env::var("AAVE_SUPPLY_TOKEN_ADDRESS")
        .unwrap_or_else(|_| "0xdac17f958d2ee523a2206206994597c13d831ec7".to_string())
    // Default: USDT
}

pub fn get_borrowed_token_address() -> String {
    env::var("AAVE_BORROWED_TOKEN_ADDRESS")
        .unwrap_or_else(|_| "0x2260fac5e5542a773aa44fbcfedf7c193bc2c599".to_string())
    // Default: wBTC
}

pub fn get_supply_token_decimals() -> u64 {
    env::var("AAVE_SUPPLY_TOKEN_DECIMALS")
        .unwrap_or_else(|_| "6".to_string()) // Default: USDT has 6 decimals
        .parse::<u64>()
        .unwrap_or(6)
}

pub fn get_borrowed_token_decimals() -> u64 {
    env::var("AAVE_BORROWED_TOKEN_DECIMALS")
        .unwrap_or_else(|_| "8".to_string()) // Default: wBTC has 8 decimals
        .parse::<u64>()
        .unwrap_or(8)
}

pub fn get_ethereum_rpc_url() -> String {
    env::var("ETHEREUM_RPC_URL").unwrap_or_else(|_| "https://mainnet.infura.io/v3/123".to_string())
}

pub fn get_ethereum_ws_url() -> String {
    env::var("ETHEREUM_WS_URL")
        .unwrap_or_else(|_| "wss://mainnet.infura.io/ws/v3/123".to_string())
}

pub fn get_liquidation_threshold() -> f64 {
    env::var("LIQUIDATION_THRESHOLD")
        .unwrap_or_else(|_| "0.89".to_string())
        .parse::<f64>()
        .unwrap_or(0.89)
}

/// Print initial configuration when application starts
pub fn print_initial_configuration() {
    println!("=== Aave Liquidator Configuration ===");
    println!("User Address to Track: {}", get_user_address_to_track());
    println!("Pool V3 Address: {}", get_pool_v3_address());
    println!(
        "Supply Token Address: {} (Decimals: {}) - Default: USDT",
        get_supply_token_address(),
        get_supply_token_decimals()
    );
    println!(
        "Borrow Token Address: {} (Decimals: {}) - Default: wBTC",
        get_borrowed_token_address(),
        get_borrowed_token_decimals()
    );
        println!("Ethereum RPC URL: {}", get_ethereum_rpc_url());
    println!("Ethereum WS URL: {}", get_ethereum_ws_url());
    println!("Liquidation Threshold: {} ({}%)", get_liquidation_threshold(), (get_liquidation_threshold() * 100.0) as i32);
    
    // Print initial position values
    match get_position_data() {
        Ok(position) => {
            println!("Initial Supplied Amount: {}", position.supplied_amount);
            println!("Initial Borrowed Amount: {}", position.borrowed_amount);
        }
        Err(e) => println!("Error getting initial position data: {}", e),
    }
    println!("=====================================");
}

pub async fn init_system() {
    // Set default RPC URL if not provided
    if env::var("ETHEREUM_RPC_URL").is_err() {
        env::set_var("ETHEREUM_RPC_URL", "https://mainnet.infura.io/v3/123");
    }

    let ethereum_rpc = get_ethereum_rpc_url();
    let _ = get_current_block_number_ethereum(&ethereum_rpc).await;
}
