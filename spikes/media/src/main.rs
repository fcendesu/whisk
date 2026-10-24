use media_spike::room::{ProbeError, ProbeReport, connect_probe};
use std::{env, process::ExitCode, time::Duration};

#[tokio::main]
async fn main() -> ExitCode {
    let args: Vec<_> = env::args().skip(1).collect();
    if args == ["--help"] {
        println!(
            "Whisk M0 native LiveKit connection probe\n\nSet LIVEKIT_URL and LIVEKIT_TOKEN, then run media-spike.\nOutputs JSON after a 2-second observation and bounded disconnect.\nConnection deadline: 10s; close deadline: 5s. No media certification."
        );
        return ExitCode::SUCCESS;
    }
    match run(&args).await {
        Ok(report) => {
            println!(
                "{}",
                serde_json::to_string(&report).expect("serializable report")
            );
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

async fn run(args: &[String]) -> Result<ProbeReport, ProbeError> {
    if !args.is_empty() {
        return Err(ProbeError::InvalidInput);
    }
    let url = env::var("LIVEKIT_URL").map_err(|_| ProbeError::InvalidInput)?;
    let token = env::var("LIVEKIT_TOKEN").map_err(|_| ProbeError::InvalidInput)?;
    let mut room = connect_probe(&url, &token).await?;
    room.observe_for(Duration::from_secs(2)).await;
    room.close().await
}
