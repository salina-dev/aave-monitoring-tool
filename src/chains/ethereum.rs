pub mod ethereum_chain {
    use crate::chains::{get_ethereum_ws_url, get_pool_v3_address, get_user_address_to_track};
    use alloy_primitives::hex;
    use alloy_primitives::{Log, B256};
    use alloy_sol_types::sol;
    use alloy_sol_types::SolEvent;
    use ethers::prelude::*;
    use log::error;
    use std::str::FromStr;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::{Arc, Mutex};

    static ETHEREUM_BLOCK_NUMBER: AtomicU64 = AtomicU64::new(0);

    // Struct to represent borrowed and supplied amounts
    #[derive(Debug, Clone)]
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

        pub fn update_supplied_amount(&mut self, new_amount: U256) {
            self.supplied_amount = new_amount;
        }

        pub fn update_borrowed_amount(&mut self, new_amount: U256) {
            self.borrowed_amount = new_amount;
        }
    }

    // Global position data that can be shared between threads
    lazy_static::lazy_static! {
        static ref POSITION_DATA: Arc<Mutex<PositionData>> = Arc::new(Mutex::new(PositionData::new()));
    }

    // Function to get current position data
    pub fn get_position_data() -> Result<PositionData, String> {
        POSITION_DATA
            .lock()
            .map(|data| data.clone())
            .map_err(|e| format!("Failed to acquire lock: {}", e))
    }

    // Function to update supplied amount
    pub fn update_supplied_amount(new_amount: U256) -> Result<(), String> {
        POSITION_DATA
            .lock()
            .map_err(|e| format!("Failed to acquire lock: {}", e))?
            .update_supplied_amount(new_amount);
        Ok(())
    }

    // Function to update borrowed amount
    pub fn update_borrowed_amount(new_amount: U256) -> Result<(), String> {
        POSITION_DATA
            .lock()
            .map_err(|e| format!("Failed to acquire lock: {}", e))?
            .update_borrowed_amount(new_amount);
        Ok(())
    }

    sol! {
        #[derive(Debug)]
        event BulkWithdraw(address indexed asset, uint256 shareAmount);
    }

    pub const SUPPLY_EVENT_TOPIC: &str =
        "2b627736bca15cd5381dcf80b0bf11fd197d01a037c52b927a881a10fb73ba61";
    pub const WITHDRAW_EVENT_TOPIC: &str =
        "3115d1449a7b732c986cba18244e897a450f61e1bb8d589cd2e69e6c8924f9f7";
    pub const REPAY_EVENT_TOPIC: &str =
        "a534c8dbe71f871f9f3530e97a74601fea17b426cae02e1c5aee42c96c784051";
    pub const BORROW_EVENT_TOPIC: &str =
        "b3d084820fb1a9decffb176436bd02558d15fac9b0ddfed8c465bc7359d7dce0";

    //all this events are from Aave Pool V3 and help us to track the supply, withdraw, repay and borrow events to calculate health factor in real time based on user activity
    sol! {
        //https://etherscan.io/tx/0xceec7b72b7c65b5a9383c961d82b4db9a04009ea42d9e95698995bd8eaaba3df#eventlog Aave: Pool V3 Supply event example
        #[derive(Debug)]
        //topic 0x2b627736bca15cd5381dcf80b0bf11fd197d01a037c52b927a881a10fb73ba61
        event Supply(address indexed reserve, address user, address indexed onBehalfOf, uint256 amount, uint16 indexed referralCode);
        //https://etherscan.io/tx/0x996d0c4031facae5f4d1958d5af5d0a8136a520d20c7bf20c526b71d40ef821e#eventlog Aave: Pool V3 Withdraw event example
        #[derive(Debug)]
        //topic 0x3115d1449a7b732c986cba18244e897a450f61e1bb8d589cd2e69e6c8924f9f7
        event Withdraw (address indexed reserve, address indexed user, address indexed to, uint256 amount);
        //https://etherscan.io/tx/0x02e072cad5cb5d913a9638c88f67959a4313c09273b9b743458f31340b104c26#eventlog Aave: Pool V2 Repay event example. It is just example, we will use the same event for both Pool V2 and V3
        #[derive(Debug)]
        //topic 0xa534c8dbe71f871f9f3530e97a74601fea17b426cae02e1c5aee42c96c784051
        event Repay (address indexed reserve, address user, address indexed repayer, uint256 amount, bool useATokens);
        //topic 0xb3d084820fb1a9decffb176436bd02558d15fac9b0ddfed8c465bc7359d7dce0
        #[derive(Debug)]
        event Borrow (address indexed reserve, address user, address indexed onBehalfOf, uint256 amount, uint8 interestRateMode, uint256 borrowRate, uint16 indexed referralCode);
    }

    pub async fn get_current_block_number_ethereum(rpc_url: &str) -> Result<(), String> {
        // Create the provider, handling any errors that may occur
        let provider = Provider::<Http>::try_from(rpc_url).map_err(|e| {
            let err_msg = format!("Failed to create provider: {}", e);
            eprintln!("{}", err_msg);
            err_msg
        })?;

        loop {
            match provider.get_block_number().await {
                Ok(res) => {
                    // Store the block number safely
                    ETHEREUM_BLOCK_NUMBER.store(res.as_u64(), Ordering::SeqCst);
                    println!("Current Ethereum block number: {}", res);
                    break;
                }
                Err(e) => {
                    // Log the error and retry after a delay
                    eprintln!("Failed to get block number: {}", e);
                    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                }
            }
        }

        Ok(())
    }

    use futures::stream::StreamExt;

    fn refresh_position_after_supply(event: Supply) -> Result<(), String> {
        let current_position = get_position_data()?;
        let event_amount = U256::from_dec_str(&event.amount.to_string())
            .expect("Failed to parse U256 from string");
        let new_supplied_amount = current_position.supplied_amount + event_amount;
        update_supplied_amount(new_supplied_amount)?;
        println!(
            "Updated supplied amount after supply event: {} -> {}",
            current_position.supplied_amount, new_supplied_amount
        );
        Ok(())
    }

    fn refresh_position_after_withdraw(event: Withdraw) -> Result<(), String> {
        let current_position = get_position_data()?;
        let event_amount = U256::from_dec_str(&event.amount.to_string())
            .expect("Failed to parse U256 from string");
        let new_supplied_amount = if current_position.supplied_amount >= event_amount {
            current_position.supplied_amount - event_amount
        } else {
            U256::from(0)
        };
        update_supplied_amount(new_supplied_amount)?;
        println!(
            "Updated supplied amount after withdraw event: {} -> {}",
            current_position.supplied_amount, new_supplied_amount
        );
        Ok(())
    }

    fn refresh_position_after_repay(event: Repay) -> Result<(), String> {
        let current_position = get_position_data()?;
        let event_amount = U256::from_dec_str(&event.amount.to_string())
            .expect("Failed to parse U256 from string");
        let new_borrowed_amount = if current_position.borrowed_amount >= event_amount {
            current_position.borrowed_amount - event_amount
        } else {
            U256::from(0)
        };
        update_borrowed_amount(new_borrowed_amount)?;
        println!(
            "Updated borrowed amount after repay event: {} -> {}",
            current_position.borrowed_amount, new_borrowed_amount
        );
        Ok(())
    }

    fn refresh_position_after_borrow(event: Borrow) -> Result<(), String> {
        let current_position = get_position_data()?;
        let event_amount = U256::from_dec_str(&event.amount.to_string())
            .expect("Failed to parse U256 from string");
        let new_borrowed_amount = current_position.borrowed_amount + event_amount;
        update_borrowed_amount(new_borrowed_amount)?;
        println!(
            "Updated borrowed amount after borrow event: {} -> {}",
            current_position.borrowed_amount, new_borrowed_amount
        );
        Ok(())
    }

    pub async fn ethereum_listening() -> Result<(), String> {
        let ws_url = get_ethereum_ws_url();

        let provider_ws = Ws::connect(&ws_url)
            .await
            .map_err(|e| format!("Failed to connect to WebSocket: {}", e))
            .map(Provider::new)?;

        let mut stream = provider_ws
            .subscribe_blocks()
            .await
            .map_err(|e| format!("Failed to subscribe to blocks: {}", e))?;

        let mut filter = Filter::new().select(BlockNumber::Latest);

        let aave_pool_v3_address = get_pool_v3_address().parse::<Address>().map_err(|e| {
            let err_msg = format!("Failed to parse contract address: {}", e);
            eprintln!("{}", err_msg);
            err_msg
        })?;

        let aave_user_address_to_track =
            get_user_address_to_track()
                .parse::<Address>()
                .map_err(|e| {
                    let err_msg = format!("Failed to parse contract address: {}", e);
                    eprintln!("{}", err_msg);
                    err_msg
                })?;

        filter.topics = [
            Some(ValueOrArray::Array(vec![
                Some(
                    hex!("2b627736bca15cd5381dcf80b0bf11fd197d01a037c52b927a881a10fb73ba61").into(),
                ), //supply event
                Some(
                    hex!("3115d1449a7b732c986cba18244e897a450f61e1bb8d589cd2e69e6c8924f9f7").into(),
                ), //withdraw event
                Some(
                    hex!("a534c8dbe71f871f9f3530e97a74601fea17b426cae02e1c5aee42c96c784051").into(),
                ), //repay event
                Some(
                    hex!("b3d084820fb1a9decffb176436bd02558d15fac9b0ddfed8c465bc7359d7dce0").into(),
                ), //borrow event
            ])),
            None,
            None,
            None,
        ];

        fn fetch_event<T: SolEvent>(
            topic: &H256,
            data: String,
            topic_str: &str,
            from_str: &str,
        ) -> Result<Option<T>, String> {
            if topic
                != &H256::from_str(topic_str).map_err(|e| format!("Failed to parse H256: {}", e))?
            {
                return Ok(None);
            }
            let log = Log::new(
                vec![B256::from_str(from_str).unwrap()],
                hex::decode(data).unwrap().into(),
            )
            .unwrap();
            let event = T::decode_log_object(&log, true)
                .map_err(|e| format!("Failed to decode log object: {}", e))?;
            Ok(Some(event))
        }

        while let Some(block) = stream.next().await {
            if let Some(_number) = block.number {
                println!("New block: {:?}", block.number);
                use chrono::Local;
                let now = Local::now();
                println!("Current local time: {}", now.format("%H:%M:%S"));

                // continue;
                match provider_ws.get_logs(&filter).await {
                    Ok(logs) => {
                        for log in logs {
                            if log.address != aave_pool_v3_address {
                                continue; // Skip logs not Aave Pool V3 but from other contracts with same events topics
                            }
                            let data_string = format!("{}", log.data);
                            let data = data_string[2..].to_string();
                            let topics = log.topics.clone();

                            let Some(topic) = topics.get(0) else {
                                error!("No topic found for log: {:?}", log);
                                continue;
                            };

                            let supply_event = fetch_event::<Supply>(
                                &topic,
                                data.clone(),
                                SUPPLY_EVENT_TOPIC,
                                &format!("0x{}", SUPPLY_EVENT_TOPIC),
                            )?;
                            // Handle Supply event
                            if let Some(event) = supply_event {
                                //convert event.user Address to H160
                                let event_user_address = H160::from_str(&event.user.to_string())
                                    .expect("Failed to parse H160 from string");
                                if event_user_address != aave_user_address_to_track {
                                    continue;
                                }
                                println!("Supply event detected: {:?}", event);
                                refresh_position_after_supply(event)?;
                                continue;
                            }

                            let withdraw_event = fetch_event::<Withdraw>(
                                &topic,
                                data.clone(),
                                WITHDRAW_EVENT_TOPIC,
                                &format!("0x{}", WITHDRAW_EVENT_TOPIC),
                            )?;
                            // Handle Withdraw event
                            if let Some(event) = withdraw_event {
                                let event_user_address = H160::from_str(&event.user.to_string())
                                    .expect("Failed to parse H160 from string");
                                if event_user_address != aave_user_address_to_track {
                                    continue;
                                }
                                println!("Withdraw event detected: {:?}", event);
                                refresh_position_after_withdraw(event)?;
                                continue;
                            }

                            let repay_event = fetch_event::<Repay>(
                                &topic,
                                data.clone(),
                                REPAY_EVENT_TOPIC,
                                &format!("0x{}", REPAY_EVENT_TOPIC),
                            )?;
                            // Handle Repay event
                            if let Some(event) = repay_event {
                                let event_user_address = H160::from_str(&event.user.to_string())
                                    .expect("Failed to parse H160 from string");
                                if event_user_address != aave_user_address_to_track {
                                    continue;
                                }
                                println!("Repay event detected: {:?}", event);
                                refresh_position_after_repay(event)?;
                                continue;
                            }

                            let borrow_event = fetch_event::<Borrow>(
                                &topic,
                                data.clone(),
                                BORROW_EVENT_TOPIC,
                                &format!("0x{}", BORROW_EVENT_TOPIC),
                            )?;
                            // Handle Borrow event
                            if let Some(event) = borrow_event {
                                let event_user_address = H160::from_str(&event.user.to_string())
                                    .expect("Failed to parse H160 from string");
                                if event_user_address != aave_user_address_to_track {
                                    continue;
                                }
                                println!("Borrow event detected: {:?}", event);
                                refresh_position_after_borrow(event)?;
                                continue;
                            }
                        }
                    }
                    Err(err) => {
                        eprintln!("Error fetching logs: {:?}", err);
                        return Err(format!("Error fetching logs: {}", err));
                    }
                }
            }
        }

        Ok(())
    }
}
