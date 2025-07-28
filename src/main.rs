mod chains;

use ethereum::ethereum_chain::{ethereum_listening, get_position_data};
use reqwest::{Client, Url};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, env};
use teloxide::{
    prelude::*,
    types::{ParseMode, Recipient},
};

use crate::chains::*;
#[cfg(test)]
mod tests;

async fn display_position_status() {
    match get_position_data() {
        Ok(position) => {
            println!("Current Position Status:");
            println!("  Supplied Amount: {}", position.supplied_amount);
            println!("  Borrowed Amount: {}", position.borrowed_amount);
        }
        Err(e) => eprintln!("Failed to get position data: {}", e),
    }
}

#[tokio::main]
async fn main() {
    //so this liquidator tracker is designed to track the 1 possition in supply sude and 1 possition in borrowed side

    //for example user supply USDT and borrowed wBTC
    //to extend it to any other token pair, we need to keep track of all the token pairs.
    //but current implementation shows the general idea of how to track the position and calculate the health factor.
    //When this bot starts it initializes the supplied and borrowed amount of user from environment variables.
    //The bot listens to the events from the Aave protocol and updates the supplied and borrowed, repays or withdraws to update the position that effect the health factor.

    dotenv::dotenv().ok();
    init_system().await;

    // Print initial configuration
    print_initial_configuration();

    // Display initial position status
    display_position_status().await;

    tokio::spawn(async {
        loop {
            let handle0 = tokio::spawn(async {
                match ethereum_listening().await {
                    Ok(_) => println!("Ethereum listening finished"),
                    Err(e) => println!("Ethereum listening failed with error: {}", e),
                };
            });
            match handle0.await {
                Ok(_) => println!("Ethereum task completed successfully."),
                Err(join_err) => {
                    if join_err.is_panic() {
                        println!("Ethereum task panicked! Restarting...");
                    } else {
                        println!("Ethereum task failed unexpectedly: {:?}", join_err);
                    }
                }
            }
            tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        }
    });

    // Spawn a task to periodically check if health factor is in liquidation range
    tokio::spawn(async {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
            let is_liquidation_range = is_health_factor_in_liquidation_range()
                .await
                .expect("Failed to check health factor");

            // Send Telegram alert if in liquidation range
            if let Err(e) = send_telegram_alert(is_liquidation_range).await {
                eprintln!("Failed to send Telegram alert: {}", e);
            }
        }
    });

    tokio::signal::ctrl_c()
        .await
        .expect("Failed to listen for ctrl_c signal");
}

pub async fn is_health_factor_in_liquidation_range() -> Result<bool, String> {
    //get supply position
    //get borrowed position
    //get price of supply and borrowed
    //calculate health factor

    let supply_position = get_position_data().expect("Failed to get supply position");

    println!("Current Position Status:");
    println!("  Supplied Amount: {}", supply_position.supplied_amount);
    println!("  Borrowed Amount: {}", supply_position.borrowed_amount);

    let borrowed_amount = supply_position.borrowed_amount;
    let supply_amount = supply_position.supplied_amount;

    //convert supply_amount and borrowed_amount to f64
    let supply_amount_f64 = supply_amount
        .to_string()
        .parse::<f64>()
        .expect("Failed to convert supply amount to f64");
    let borrowed_amount_f64 = borrowed_amount
        .to_string()
        .parse::<f64>()
        .expect("Failed to convert borrowed amount to f64");

    let supply_price = get_price(get_supply_token_address())
        .await
        .expect("Failed to get supply price")
        .expect("Failed to get supply price");
    let borrowed_price = get_price(get_borrowed_token_address())
        .await
        .expect("Failed to get borrowed price")
        .expect("Failed to get borrowed price");

    let supply_in_usd =
        supply_price.price * supply_amount_f64 / 10_f64.powf(get_supply_token_decimals() as f64);
    let borrowed_in_usd = borrowed_price.price * borrowed_amount_f64
        / 10_f64.powf(get_borrowed_token_decimals() as f64);

    let health_factor = borrowed_in_usd / supply_in_usd;
    let liquidation_threshold = get_liquidation_threshold();
    if health_factor > liquidation_threshold {
        // alert
        return Ok(true);
    }

    Ok(false)
}

/// Send a Telegram alert when liquidation range is detected
async fn send_telegram_alert(is_liquidation_range: bool) -> Result<(), Box<dyn std::error::Error>> {
    // Get bot token and chat ID from environment variables
    let bot_token =
        env::var("TELEGRAM_BOT_TOKEN").expect("TELEGRAM_BOT_TOKEN environment variable not set");
    let chat_id =
        env::var("TELEGRAM_CHAT_ID").expect("TELEGRAM_CHAT_ID environment variable not set");

    let bot = Bot::new(bot_token);
    let chat_id = chat_id.parse::<u64>()?;

    let user_id = UserId(chat_id);
    let recipient = Recipient::from(user_id);

    if is_liquidation_range {
        let message = format!(
            "🚨 *LIQUIDATION ALERT* 🚨\n\n\
            *Address:* `{}`\n\
            *Supply Token:* `{}` \\(Decimals: {}\\)\n\
            *Borrow Token:* `{}` \\(Decimals: {}\\)\n\n\
            Your Aave position is now in liquidation range\\!\n\n\
            Please check your position immediately and consider:\n\
            • Repaying some debt\n\
            • Adding more collateral\n\
            • Closing the position\n\n\
            Health factor is below {}\\.\n\
            \\(Borrowed value is {}% of supply value\\)",
            get_user_address_to_track(),
            get_supply_token_address(),
            get_supply_token_decimals(),
            get_borrowed_token_address(),
            get_borrowed_token_decimals(),
            get_liquidation_threshold(),
            (get_liquidation_threshold() * 100.0) as i32
        );

        bot.send_message(recipient, message)
            .parse_mode(ParseMode::MarkdownV2)
            .await?;
    }

    Ok(())
}

pub async fn get_price(smart_contract: String) -> Result<Option<PriceResult>, reqwest::Error> {
    //todo: read api key from env var.
    let api_key = "secret_sk_1234567890";
    let mut url = Url::parse("https://api.simplehash.com/api/v0/fungibles/assets").unwrap();

    //if it starts with 0x then it is eth
    let smart_contract = if smart_contract.starts_with("0x") {
        format!("ethereum.{}", smart_contract)
    } else {
        format!("solana.{}", smart_contract)
    };

    let mut query_params = HashMap::new();
    query_params.insert("fungible_ids", smart_contract.clone());
    query_params.insert("include_prices", "1".to_string());

    url.set_query(Some(
        &query_params
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect::<Vec<_>>()
            .join("&"),
    ));

    let client = Client::new();
    let resp = client
        .get(url)
        .header("X-API-KEY", api_key)
        .header("Accepts", "application/json")
        .send()
        .await
        .expect("Failed to send request")
        .text()
        .await?;

    let parsed: Result<SimplehashPriceResp, _> = serde_json::from_str(&resp);

    if parsed.is_err() {
        log::warn!(
            "Failed parsed response simplehash for address {}",
            smart_contract
        );
        log::warn!("Response: {:?}", resp);
        return Ok(None);
    }
    let parsed = parsed.unwrap();

    let high_precision_values: Vec<f64> = parsed
        .prices
        .iter()
        .filter_map(|price| price.value_usd_string_high_precision.parse::<f64>().ok())
        .collect();

    if !high_precision_values.is_empty() {
        let mut avg = get_avg(high_precision_values.clone());
        if avg.is_none() {
            avg = Some(high_precision_values[0]);
        }

        if avg.is_none() {
            log::warn!("Failed to calculate average for address {}", smart_contract);
            return Ok(None);
        }

        return Ok(Some(PriceResult {
            price: avg.expect("Should never be None"),
            decimals: parsed.decimals,
            symbol: parsed.symbol,
        }));
    } else {
        log::warn!("No prices for address {}", smart_contract);
    }

    return Ok(None);
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Prices {
    pub marketplace_id: String,
    pub marketplace_name: String,
    pub value_usd_cents: u64,
    pub value_usd_string: String,
    pub value_usd_string_high_precision: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SimplehashPriceResp {
    pub decimals: u64,
    pub prices: Vec<Prices>,
    pub symbol: String,
}

fn get_avg(prices: Vec<f64>) -> Option<f64> {
    let mean: f64 = prices.iter().sum::<f64>() / prices.len() as f64;

    let new_v: Vec<f64> = prices
        .into_iter()
        .filter(|&price| (price - mean).abs() <= mean)
        .collect();

    if new_v.is_empty() {
        return None;
    }
    let avg = new_v.iter().sum::<f64>() / new_v.len() as f64;
    Some(avg)
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PriceResult {
    pub symbol: String,
    pub price: f64,
    pub decimals: u64,
}
