use clap::Parser;
use gettextrs::TextDomain;
use irc::client::prelude::*;
use std::str::FromStr;
use std::sync::Arc;
use tokio::signal::unix::{SignalKind, signal};
use tokio::sync::Notify;
use tokio_stream::StreamExt;
use tokio_stream::wrappers::SignalStream;

use crate::bot::Script;

mod bot;
mod error;

/// A simple IRC bot
#[derive(Parser, Debug)]
#[command(version)]
struct Args {
    /// Server address with optional port (e.g., irc.example.com:6667)
    server: String,

    /// Comma-separated list of channels to join
    channels: String,

    /// Bot nickname
    nickname: String,

    /// Optional NickServ password
    nickserv_password: Option<String>,
}

fn parse_server_address(server: &str) -> (String, u16) {
    if let Some((host, port_str)) = server.split_once(':')
        && let Ok(port) = u16::from_str(port_str)
    {
        return (host.to_string(), port);
    }
    (server.to_string(), 6667)
}

fn textdomain_dir() -> Option<String> {
    // Start in the executable directory, walk up to find the "gettext" directory.
    let mut dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()))
        .unwrap_or_else(|| std::env::current_dir().unwrap());
    loop {
        let gettext_dir = dir.join("gettext");
        if gettext_dir.is_dir() {
            return Some(gettext_dir.to_string_lossy().into_owned());
        }
        if !dir.pop() {
            break;
        }
    }
    None
}

#[tokio::main]
async fn main() {
    if let Some(dir) = textdomain_dir() {
        // Ignore errors and use untranslated strings if it fails.
        let _ = TextDomain::new("japanese_tools")
            .skip_system_data_paths()
            .push(&dir)
            .init();
    }

    let args = Args::parse();
    let (server, port) = parse_server_address(&args.server);
    println!("Connecting to {}:{} as {}", server, port, args.nickname);

    let shutdown_notify = Arc::new(Notify::new());
    let shutdown_notify_clone = shutdown_notify.clone();
    tokio::spawn(async move {
        let term_stream = SignalStream::new(signal(SignalKind::terminate()).unwrap());
        let int_stream = SignalStream::new(signal(SignalKind::interrupt()).unwrap());
        let mut stream = term_stream.merge(int_stream);
        let mut count = 0;
        while let Some(()) = stream.next().await {
            count += 1;
            if count == 1 {
                println!("Shutting down... (Press Ctrl+C again to force exit)");
                shutdown_notify_clone.notify_one();
            } else {
                std::process::exit(1);
            }
        }
    });

    let channels = args
        .channels
        .split(',')
        .map(|s| s.to_string())
        .collect::<Vec<_>>();
    if channels.is_empty() {
        eprintln!("No channels specified");
        std::process::exit(1);
    }
    let main_channel = channels[0].to_string();

    let config = irc::client::data::Config {
        nickname: Some(args.nickname.clone()),
        nick_password: args.nickserv_password,
        server: Some(server.clone()),
        port: Some(port),
        use_tls: Some(false),
        channels,
        ..Default::default()
    };

    match run_bot(config, &main_channel, shutdown_notify).await {
        Ok(_) => println!("Exiting..."),
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }
}

async fn run_bot(
    config: irc::client::data::Config,
    main_channel: &str,
    shutdown_notify: Arc<Notify>,
) -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::from_config(config).await?;
    client.identify()?;
    // Set bot mode.
    client.send_mode(
        client.current_nickname(),
        &[Mode::Plus(UserMode::Unknown('B'), None)],
    )?;

    let scripts = vec![
        Script::new("ai", "../ai/ai"),
        Script::new("cdecl", "../cdecl/c.sh"),
        Script::new("c++decl", "../cdecl/c++.sh"),
        Script::new("rtk", "../rtk/rtk.sh"),
        Script::new("romaji", "../romaji/romaji.sh"),
        Script::new("kanjidic", "../kanjidic/kanjidic.sh"),
        Script::new("kana", "../reading/read.py"),
        Script::new("hira", "../kana/hira.sh"),
        Script::new("kata", "../kana/kata.sh"),
        Script::new("ja", "../jmdict/jm.sh"),
        Script::new("wa", "../jmdict/wa.sh"),
        Script::new("audio", "../audio/find_audio.sh"),
        Script::new_with_timers("quiz", "../reading_quiz/quiz.sh"),
        Script::new_with_timers("kuiz", "../kumitate_quiz/kuiz.sh"),
        Script::new("calc", "../mueval/run.sh"),
        Script::new("tok", "../tokenizer/tokenizer"),
        Script::new("type", "../mueval/type.sh"),
        Script::new("utf", "../compare_encoding/compare_encoding.sh"),
        Script::new("lhc", "../lhc/lhc_info.sh"),
    ];

    let mut bot = bot::Bot::new(client, main_channel, scripts);
    let mut stream = bot.stream()?;
    loop {
        tokio::select! {
            _ = shutdown_notify.notified() => bot.quit(None)?,
            message = stream.next() => {
                match message {
                    Some(Ok(message)) => bot.handle_message(&message)?,
                    Some(Err(e)) => {
                        return Err(Box::new(e));
                    }
                    None => break,
                }
            }
            _ = bot.next_timer() => bot.run_timed_command()?,
            _ = bot.next_background_job() => bot.run_background_job()?,
            _ = bot.next_ping() => bot.send_ping()?,
        }
    }

    Ok(())
}
