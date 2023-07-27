use metrics::{register_counter, describe_counter, increment_counter, Unit};
use poirot_core::{decode::Parser, stats::ParserStatsLayer};
use reth_primitives::{BlockId, BlockNumberOrTag::Number};
use reth_rpc_types::trace::parity::{TraceResultsWithTransactionHash, TraceType};
use reth_tracing::TracingClient;
use tracing::{Level, info, Span, span, instrument};
use tracing_futures::Instrument;
use tracing_subscriber::{
    prelude::__tracing_subscriber_SubscriberExt, util::SubscriberInitExt, Registry, EnvFilter, Layer,
};
use colored::Colorize;

//Std
use std::{collections::HashSet, env, error::Error, path::Path};

fn main() {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .thread_stack_size(8 * 1024 * 1024)
        .build()
        .unwrap();
    let filter =
    EnvFilter::builder().with_default_directive(Level::INFO.into()).from_env_lossy();

    let subscriber =
        Registry::default().with(tracing_subscriber::fmt::layer().with_filter(filter)).with(ParserStatsLayer);

    tracing::subscriber::set_global_default(subscriber)
        .expect("Could not set global default subscriber");

    match runtime.block_on(run(runtime.handle().clone())) {
        Ok(()) => println!("Success!"),
        Err(e) => {
            eprintln!("Error: {:?}", e);

            let mut source: Option<&dyn Error> = e.source();
            while let Some(err) = source {
                eprintln!("Caused by: {:?}", err);
                source = err.source();
            }
        }
    }
}

#[instrument(skip_all)]
async fn run(handle: tokio::runtime::Handle) -> Result<(), Box<dyn Error>> {
    let db_path = match env::var("DB_PATH") {
        Ok(path) => path,
        Err(_) => return Err(Box::new(std::env::VarError::NotPresent)),
    };

    println!("found db path");

    let key = match env::var("ETHERSCAN_API_KEY") {
        Ok(key) => key,
        Err(_) => return Err(Box::new(std::env::VarError::NotPresent)),
    };
    println!("found etherscan api key");

    let tracer = TracingClient::new(Path::new(&db_path), handle);

    let mut parser = Parser::new(key.clone());

    let (start_block, end_block) = (17679852, 17679854);
    for i in start_block..end_block {
        let block_trace: Vec<TraceResultsWithTransactionHash> = trace_block(&tracer, i).await.unwrap();
        let action = parser.parse_block(i, block_trace).await;
    }
    info!(target: "poirot::stats", "Finished Parsing Blocks: {}", format!("{} to {}", start_block, end_block).bright_blue().bold());

    Ok(())
}

async fn trace_block(
    tracer: &TracingClient,
    block_number: u64,
) -> Result<Vec<TraceResultsWithTransactionHash>, Box<dyn Error>> {
    let mut trace_type = HashSet::new();
    trace_type.insert(TraceType::Trace);

    let parity_trace = tracer
        .trace
        .replay_block_transactions(BlockId::Number(Number(block_number)), trace_type)
        .await
        .map_err(|e| Box::new(e) as Box<dyn Error>)?
        .unwrap();

    Ok(parity_trace)
}
