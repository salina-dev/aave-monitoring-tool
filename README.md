# Aave Liquidator

A Rust application that monitors Aave positions in real-time and sends Telegram alerts when positions are in liquidation range.

## Features

- **Real-time monitoring** of Aave supply and borrow positions via WebSocket
- **Two-token system**: Monitors any token pair (supply side + borrow side)
- **Configurable tokens**: Support for any ERC-20 tokens with configurable decimals
- **Live event tracking**: Subscribes to Aave events (Supply, Borrow, Repay, Withdraw)
- **Automatic position updates**: Updates position data in real-time based on blockchain events
- **Telegram alerts**: Sends immediate alerts when health factor drops below configurable threshold
- **Ethereum integration**: Uses Infura API RPC with WebSocket subscription
- **Fast price aggregation**: Uses SimpleHash API for real-time price updates

## Token Configuration

The bot is designed to track **any 2 tokens**:
- **1 token on supply side** (what you deposit as collateral)
- **1 token on borrow side** (what you borrow)

### Configurable Parameters
- **Token addresses**: Set via environment variables
- **Token decimals**: Configurable for each token
- **Default setup**: USDT (supply) + wBTC (borrow)

### Price Aggregation
The bot uses [SimpleHash API](https://api.simplehash.com) for price aggregation:
- **Faster than on-chain**: Updates prices faster than blockchain price feeds
- **Multi-platform average**: Aggregates prices from multiple exchanges/platforms
- **Real-time updates**: Provides current market prices for accurate health factor calculation
- **Smart contract integration**: Passes token contract addresses to get accurate pricing

### Liquidation Threshold
The bot monitors your position's health factor and alerts when it approaches liquidation:
- **Liquidation Threshold Calculation**: `borrowed_value_in_usd / supplied_value_in_usd`
- **Liquidation Threshold**: Configurable via `LIQUIDATION_THRESHOLD` environment variable (default: 0.89)
- **Alert Trigger**: When borrowed value exceeds 89% of supply value (default), the bot sends alerts
- **Example**: If you have $1000 in supply and $900 in borrow, health factor = 0.9 (90%), which would trigger an alert
- **Safety Margin**: The default 0.89 threshold provides an 1% safety margin before actual liquidation

### Extending to Multiple Token Pairs
To support multiple token pairs simultaneously, the source code would need to be extended. This involves:
- Adding support for multiple position tracking
- Extending the event monitoring system to track more then 2 assets
- Modifying the health factor calculation logic
- Updating the alert system for multiple positions

While not a significant amount of work, it requires careful consideration of the architecture to maintain real-time performance.

## Build & Run

### 1. Build the Project

```bash
cargo build -p aave-liquidator-alarm-bot
```

This will compile the project and all its dependencies. Make sure you have set up your `.env` file as described above.

### 2. Run the Bot

```bash
cargo run -p aave-liquidator-alarm-bot
```

The bot will start, print its configuration, and begin monitoring your Aave position.

## Toolchain Version

This repository includes a `rust-toolchain.toml` file.

**Purpose:**
- It pins the Rust toolchain to a specific version to ensure compatibility with the latest versions of the Telegram (`teloxide`) crates and other dependencies.
- If you encounter build errors related to the Rust version, make sure you are using the toolchain specified in `rust-toolchain.toml`.
- Rustup will automatically use the correct version if you have it installed.


## How It Works

1. **Initialization**: Bot starts with your specified initial supply and borrow amounts
2. **WebSocket Connection**: Connects to Infura WebSocket API to monitor Ethereum blocks
3. **Event Monitoring**: Listens for specific Aave protocol events:
   - `Supply`: Updates supplied amount when you deposit tokens
   - `Borrow`: Updates borrowed amount when you borrow tokens
   - `Repay`: Updates borrowed amount when you repay tokens
   - `Withdraw`: Updates supplied amount when you withdraw tokens
4. **Real-time Updates**: Position data is updated immediately when events are detected
5. **Price Aggregation**: Fetches real-time prices from SimpleHash API for accurate calculations
6. **Health Factor Calculation**: Continuously calculates health factor based on current prices and position
7. **Alert System**: Sends Telegram alerts with specific address and token information when liquidation risk is detected

## Alert Message

When a liquidation alert is triggered, you'll receive a Telegram message with:
- 🚨 Warning emoji and clear alert title
- **Specific address** being monitored
- **Token addresses** with their decimals
- Instructions on what to do (repay debt, add collateral, close position)
- **Health factor information** with current threshold and percentage
- **Safety warning** showing borrowed value as percentage of supply value

## Use Case

This bot is specifically designed for users who:
- **Supply any token** to Aave protocol as collateral
- **Borrow any token** against their collateral
- Want real-time monitoring of their position's liquidation risk
- Need immediate alerts when health factor becomes critical
- Require fast price updates for accurate health factor calculation

## Architecture

### Token Configuration
- **Supply Token**: Configurable via environment variables (default: USDT)
- **Borrow Token**: Configurable via environment variables (default: wBTC)
- **Tracked Address**: Your wallet address to monitor

### Real-time Monitoring
The bot uses Infura WebSocket API to:
1. Subscribe to new blocks
2. Monitor specific Aave event topics
3. Track events: Supply, Borrow, Repay, Withdraw
4. Update position data in real-time based on events

### Price Aggregation System
- **API**: SimpleHash (https://api.simplehash.com)
- **Method**: Pass smart contract address to get aggregated prices
- **Advantage**: Faster updates than on-chain price feeds
- **Coverage**: Multiple exchange platforms for accurate pricing

### Aave Event Topics

The bot monitors these specific Aave Pool V3 event topics:

#### Supply Event
- **Topic**: `0x2b627736bca15cd5381dcf80b0bf11fd197d01a037c52b927a881a10fb73ba61`
- **Event**: `Supply(address indexed reserve, address user, address indexed onBehalfOf, uint256 amount, uint16 indexed referralCode)`
- **Example**: [Etherscan Transaction](https://etherscan.io/tx/0xceec7b72b7c65b5a9383c961d82b4db9a04009ea42d9e95698995bd8eaaba3df#eventlog)
- **Effect**: Increases supplied amount when user deposits tokens

#### Withdraw Event
- **Topic**: `0x3115d1449a7b732c986cba18244e897a450f61e1bb8d589cd2e69e6c8924f9f7`
- **Event**: `Withdraw(address indexed reserve, address indexed user, address indexed to, uint256 amount)`
- **Example**: [Etherscan Transaction](https://etherscan.io/tx/0x996d0c4031facae5f4d1958d5af5d0a8136a520d20c7bf20c526b71d40ef821e#eventlog)
- **Effect**: Decreases supplied amount when user withdraws tokens

#### Repay Event
- **Topic**: `0xa534c8dbe71f871f9f3530e97a74601fea17b426cae02e1c5aee42c96c784051`
- **Event**: `Repay(address indexed reserve, address user, address indexed repayer, uint256 amount, bool useATokens)`
- **Example**: [Etherscan Transaction](https://etherscan.io/tx/0x02e072cad5cb5d913a9638c88f67959a4313c09273b9b743458f31340b104c26#eventlog)
- **Effect**: Decreases borrowed amount when user repays debt

#### Borrow Event
- **Topic**: `0xb3d084820fb1a9decffb176436bd02558d15fac9b0ddfed8c465bc7359d7dce0`
- **Event**: `Borrow(address indexed reserve, address user, address indexed onBehalfOf, uint256 amount, uint8 interestRateMode, uint256 borrowRate, uint16 indexed referralCode)`
- **Effect**: Increases borrowed amount when user borrows tokens

## Setup

### Environment Variables

Create a `.env` file in the project root with the following variables:

```env
# Telegram Bot Configuration
# Get your bot token from @BotFather on Telegram
TELEGRAM_BOT_TOKEN=your_bot_token_here

# Get your chat ID by sending a message to your bot and checking the chat_id
# You can use @userinfobot to get your chat ID
TELEGRAM_CHAT_ID=your_chat_id_here

# Initial Position Values (required on bot startup)
# These should reflect your current Aave position values
INITIAL_SUPPLIED_AMOUNT=your_initial_supply_amount
INITIAL_BORROWED_AMOUNT=your_initial_borrow_amount

# Aave Configuration
# User address to track (your wallet address)
AAVE_USER_ADDRESS_TO_TRACK=0xBDD3B59416Fc0263354953aeeFC51Ba3A94E134e

# Aave Pool V3 contract address
AAVE_POOL_V3_ADDRESS=0x87870Bca3F3fD6335C3F4ce8392D69350B4fA4E2

# Token Configuration (Configurable for any ERC-20 tokens)
# Supply token address (what you're depositing as collateral)
AAVE_SUPPLY_TOKEN_ADDRESS=0xdac17f958d2ee523a2206206994597c13d831ec7
# Borrow token address (what you're borrowing)
AAVE_BORROWED_TOKEN_ADDRESS=0x2260fac5e5542a773aa44fbcfedf7c193bc2c599

# Token decimals (configurable for each token)
AAVE_SUPPLY_TOKEN_DECIMALS=6
AAVE_BORROWED_TOKEN_DECIMALS=8

# Liquidation Threshold Configuration
# Health factor threshold for liquidation alerts (default: 0.89 = 89%)
# When borrowed value exceeds this percentage of supply value, alerts are triggered
LIQUIDATION_THRESHOLD=0.89

# Ethereum RPC Configuration
# Replace with your own Infura API key or other RPC provider
ETHEREUM_RPC_URL=https://mainnet.infura.io/v3/YOUR_INFURA_API_KEY
ETHEREUM_WS_URL=wss://mainnet.infura.io/ws/v3/YOUR_INFURA_API_KEY
```

### Initial Position Setup

**Important**: When the bot starts, you must specify the initial position values in environment variables:

- `INITIAL_SUPPLIED_AMOUNT`: Your current supply amount in Aave
- `INITIAL_BORROWED_AMOUNT`: Your current borrow amount in Aave

These values serve as the starting point, and the bot will update them in real-time based on blockchain events.

### Telegram Bot Setup

1. Create a new bot using [@BotFather](https://t.me/botfather) on Telegram
2. Get your bot token from BotFather
3. Start a conversation with your bot
4. Get your chat ID by:
   - Sending a message to your bot
   - Using [@userinfobot](https://t.me/userinfobot) to get your chat ID
   - Or checking the bot's webhook logs

### Running the Application

```bash
cargo run
```

The application will:
- Initialize with your specified initial position values
- Subscribe to Ethereum blocks via Infura WebSocket
- Monitor Aave events in real-time
- Update position data automatically based on Supply/Borrow/Repay/Withdraw events
- Calculate health factor every 2 seconds using real-time prices from SimpleHash
- Send Telegram alerts when health factor drops below configurable threshold
- Continue monitoring until interrupted with control+C (Ctrl+C) 


