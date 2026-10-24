use media_spike::room::{ProbeError, ProbeReport, connect_probe};
use std::{env, process::ExitCode, time::Duration};

#[tokio::main]
async fn main() -> ExitCode {
    let args: Vec<_> = env::args().skip(1).collect();
    if args == ["--help"] {
        println!(
            "Whisk M0 native LiveKit connection probe\n\nSet LIVEKIT_URL and LIVEKIT_TOKEN, then run media-spike.\nOutputs JSON after a 2-second observation and bounded disconnect.\nConnection deadline: 10s; close deadline: 5s. No media certification.\nUse --loopback with LIVEKIT_PUBLISH_TOKEN and LIVEKIT_TOKEN for a 3-second native H.264 SFU test.\nUse --switch with LIVEKIT_SECOND_PUBLISH_TOKEN to test two publishers, viewer focus switching and generated audio.\nUse --profile 720p30|1080p60|1440p60 PHASE_SECONDS for experimental profile measurements (1-300 seconds per phase).\nUse --relay-only to require a TURN relay for the connection probe."
        );
        return ExitCode::SUCCESS;
    }
    let profile_run = match args.as_slice() {
        [flag, profile, seconds] if flag == "--profile" => {
            let profile = match profile.as_str() {
                "720p30" => Some(media_spike::codec::VideoProfile::P720F30),
                "1080p60" => Some(media_spike::codec::VideoProfile::P1080F60),
                "1440p60" => Some(media_spike::codec::VideoProfile::P1440F60),
                _ => None,
            };
            profile.zip(seconds.parse::<u64>().ok())
        }
        _ => None,
    };
    if args == ["--switch"] || profile_run.is_some() {
        let (profile, phase_seconds) =
            profile_run.unwrap_or((media_spike::codec::VideoProfile::P720F30, 2));
        let url = env::var("LIVEKIT_URL").unwrap_or_default();
        let first = env::var("LIVEKIT_PUBLISH_TOKEN").unwrap_or_default();
        let second = env::var("LIVEKIT_SECOND_PUBLISH_TOKEN").unwrap_or_default();
        let viewer = env::var("LIVEKIT_TOKEN").unwrap_or_default();
        match media_spike::switching::run_profile_probe(
            &url,
            &first,
            &second,
            &viewer,
            Duration::from_secs(phase_seconds),
            profile,
        )
        .await
        {
            Ok(report) => {
                println!(
                    "{}",
                    serde_json::to_string(&report).expect("serializable switching report")
                );
                return ExitCode::SUCCESS;
            }
            Err(error) => {
                eprintln!("{error}");
                return ExitCode::FAILURE;
            }
        }
    }
    if args == ["--loopback"] {
        let url = env::var("LIVEKIT_URL").unwrap_or_default();
        let publisher = env::var("LIVEKIT_PUBLISH_TOKEN").unwrap_or_default();
        let viewer = env::var("LIVEKIT_TOKEN").unwrap_or_default();
        match media_spike::loopback::run_loopback(&url, &publisher, &viewer, Duration::from_secs(3))
            .await
        {
            Ok(report) => {
                println!(
                    "{}",
                    serde_json::to_string(&report).expect("serializable media report")
                );
                return ExitCode::SUCCESS;
            }
            Err(error) => {
                eprintln!("{error}");
                return ExitCode::FAILURE;
            }
        }
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
    let relay_only = args == ["--relay-only"];
    if !args.is_empty() && !relay_only {
        return Err(ProbeError::InvalidInput);
    }
    let url = env::var("LIVEKIT_URL").map_err(|_| ProbeError::InvalidInput)?;
    let token = env::var("LIVEKIT_TOKEN").map_err(|_| ProbeError::InvalidInput)?;
    let mut room = if relay_only {
        media_spike::room::connect_probe_with_policy(
            &url,
            &token,
            Duration::from_secs(10),
            media_spike::room::IcePolicy::RelayOnly,
        )
        .await?
    } else {
        connect_probe(&url, &token).await?
    };
    room.observe_for(Duration::from_secs(2)).await;
    room.close().await
}
