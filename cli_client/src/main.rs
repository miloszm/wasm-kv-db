use clap::{Parser, Subcommand};
use reqwest;
use rmp_serde::{Deserializer, Serializer};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::io::Cursor;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenericRequest {
    pub tenant_id: String,
    pub reducer_name: String,
    pub reducer_args: Vec<u8>, // MessagePack serialized args
    pub caller_id: String,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuyTicketArgs {
    pub raffle_id: String,
    pub user_id: String,
    pub quantity: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateRaffleArgs {
    pub raffle_id: String,
    pub total_tickets: u32,
    pub end_time: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DrawWinnerArgs {
    pub raffle_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetRaffleStatusArgs {
    pub raffle_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuyTicketResult {
    pub success: bool,
    pub message: String,
    pub tickets_left: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateRaffleResult {
    pub success: bool,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DrawWinnerResult {
    pub success: bool,
    pub winner: Option<String>,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaffleStatusResult {
    pub raffle_id: String,
    pub tickets_left: u32,
    pub total_entries: u32,
    pub winner: Option<String>,
    pub is_closed: bool,
}

fn to_msgpack<T: Serialize>(value: &T) -> Vec<u8> {
    let mut buf = Vec::new();
    value.serialize(&mut Serializer::new(&mut buf)).unwrap();
    buf
}

fn from_msgpack<T: DeserializeOwned>(data: &[u8]) -> T {
    let mut deserializer = Deserializer::new(Cursor::new(data));
    T::deserialize(&mut deserializer).unwrap()
}

#[derive(Parser)]
#[command(name = "raffle-cli")]
#[command(about = "Raffle system client", long_about = None)]
struct Cli {
    #[arg(long, default_value = "http://localhost:8080")]
    host: String,

    #[arg(short, long, default_value = "t01")]
    tenant: String,

    #[arg(short, long, default_value = "admin")]
    caller: String,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Create a new raffle
    Create {
        #[arg(short, long)]
        raffle_id: String,

        #[arg(short, long)]
        total_tickets: u32,

        #[arg(short, long, default_value = "0")]
        end_time: u64,
    },

    /// Buy a ticket for a raffle
    Buy {
        #[arg(short, long)]
        raffle_id: String,

        #[arg(short, long, default_value = "")]
        user_id: String,

        #[arg(short, long, default_value = "1")]
        quantity: u32,
    },

    /// Draw a winner for a raffle
    Draw {
        #[arg(short, long)]
        raffle_id: String,
    },

    /// Get raffle status
    Status {
        #[arg(short, long)]
        raffle_id: String,
    },

    /// List all raffles
    List,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    let client = reqwest::Client::new();
    let host_url = format!("{}/kvexec", cli.host);

    match cli.command {
        Commands::Create {
            raffle_id,
            total_tickets,
            end_time,
        } => {
            let args = CreateRaffleArgs {
                raffle_id,
                total_tickets,
                end_time,
            };
            let result = call_reducer::<CreateRaffleResult>(
                &client,
                &host_url,
                &cli.tenant,
                &cli.caller,
                "create_raffle",
                &to_msgpack(&args),
            )
            .await?;

            println!("{}", result.message);
        }

        Commands::Buy {
            raffle_id,
            user_id: _,
            quantity,
        } => {
            let args = BuyTicketArgs {
                raffle_id,
                user_id: cli.caller.clone(),
                quantity,
            };
            let result = call_reducer::<BuyTicketResult>(
                &client,
                &host_url,
                &cli.tenant,
                &cli.caller,
                "buy_ticket",
                &to_msgpack(&args),
            )
            .await?;

            if result.success {
                println!("{}", result.message);
                println!("Tickets remaining: {}", result.tickets_left);
            } else {
                println!("{}", result.message);
            }
        }

        Commands::Draw { raffle_id } => {
            let args = DrawWinnerArgs { raffle_id };
            let result = call_reducer::<DrawWinnerResult>(
                &client,
                &host_url,
                &cli.tenant,
                &cli.caller,
                "draw_winner",
                &to_msgpack(&args),
            )
            .await?;

            if result.success {
                if let Some(winner) = result.winner {
                    println!("Winner: {}", winner);
                }
            } else {
                println!("{}", result.message);
            }
        }

        Commands::Status { raffle_id } => {
            let args = GetRaffleStatusArgs {
                raffle_id: raffle_id.clone(),
            };
            let result = call_reducer::<RaffleStatusResult>(
                &client,
                &host_url,
                &cli.tenant,
                &cli.caller,
                "get_raffle_status",
                &to_msgpack(&args),
            )
            .await?;

            println!("   Raffle: {}", result.raffle_id);
            println!("   Tickets remaining: {}", result.tickets_left);
            println!("   Total entries: {}", result.total_entries);
            println!(
                "   Status: {}",
                if result.is_closed { "Closed" } else { "Open" }
            );
            if let Some(winner) = result.winner {
                println!("   Winner: {}", winner);
            }
        }

        Commands::List => {
            // This calls a reducer that returns a list of raffle IDs
            // For simplicity, we'll just call a "list_raffles" reducer
            let result = call_reducer::<Vec<String>>(
                &client,
                &host_url,
                &cli.tenant,
                &cli.caller,
                "list_raffles",
                &[],
            )
            .await?;

            println!("📋 Raffles:");
            for raffle_id in result {
                println!("   - {}", raffle_id);
            }
        }
    }

    Ok(())
}

async fn call_reducer<T: DeserializeOwned>(
    client: &reqwest::Client,
    host_url: &str,
    tenant_id: &str,
    caller_id: &str,
    reducer_name: &str,
    args_bytes: &[u8],
) -> Result<T, Box<dyn std::error::Error>> {
    let request = GenericRequest {
        tenant_id: tenant_id.to_string(),
        reducer_name: reducer_name.to_string(),
        reducer_args: args_bytes.to_vec(),
        caller_id: caller_id.to_string(),
        timestamp: 0,
    };

    let request_bytes = to_msgpack(&request);

    let response = client
        .post(host_url)
        .header("Content-Type", "application/msgpack")
        .body(request_bytes)
        .send()
        .await?;

    if !response.status().is_success() {
        let error_text = response.text().await?;
        return Err(format!("Host error: {}", error_text).into());
    }

    let response_bytes = response.bytes().await?;

    let result: T = from_msgpack(&response_bytes);
    Ok(result)
}

/*

$./target/debug/raffle-cli --caller admin create --raffle-id raffle_125 --total-tickets 100
Raffle raffle_125 created with 100 tickets
$./target/debug/raffle-cli --caller user_001 buy --raffle-id raffle_125 --quantity 1
Purchased 1 ticket(s)! 99 remaining
Tickets remaining: 99
$./target/debug/raffle-cli --caller user_002 buy --raffle-id raffle_125 --quantity 1
Purchased 1 ticket(s)! 98 remaining
Tickets remaining: 98
$./target/debug/raffle-cli --caller user_003 buy --raffle-id raffle_125 --quantity 1
Purchased 1 ticket(s)! 97 remaining
Tickets remaining: 97
$./target/debug/raffle-cli --caller admin draw --raffle-id raffle_125
Winner: user_003

*/

