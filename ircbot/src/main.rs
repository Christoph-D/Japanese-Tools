use clap::Parser;
use irc::client::prelude::*;
use std::str::FromStr;
use std::sync::Arc;
use tokio::signal::unix::{SignalKind, signal};
use tokio::sync::Notify;
use tokio_stream::StreamExt;
use tokio_stream::wrappers::SignalStream;

mod bot;

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
    if let Some((host, port_str)) = server.split_once(':') {
        if let Ok(port) = u16::from_str(port_str) {
            return (host.to_string(), port);
        }
    }
    (server.to_string(), 6667)
}

#[tokio::main]
async fn main() {
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

    let config = irc::client::data::Config {
        nickname: Some(args.nickname.clone()),
        nick_password: args.nickserv_password,
        server: Some(server.clone()),
        port: Some(port),
        use_tls: Some(false),
        channels: args.channels.split(',').map(|s| s.to_string()).collect(),
        ..Default::default()
    };

    match run_bot(config, shutdown_notify).await {
        Ok(_) => println!("Exiting..."),
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }
}

async fn run_bot(
    config: irc::client::data::Config,
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
        ("ai", "../ai/ai"),
        ("cdecl", "../cdecl/c.sh"),
        ("c++decl", "../cdecl/c++.sh"),
        ("rtk", "../rtk/rtk.sh"),
        ("romaji", "../romaji/romaji.sh"),
        ("kanjidic", "../kanjidic/kanjidic.sh"),
        ("kana", "../reading/read.py"),
        ("hira", "../kana/hira.sh"),
        ("kata", "../kana/kata.sh"),
        ("ja", "../jmdict/jm.sh"),
        ("wa", "../jmdict/wa.sh"),
        ("audio", "../audio/find_audio.sh"),
        ("quiz", "../reading_quiz/quiz.sh"),
        ("kuiz", "../kumitate_quiz/kuiz.sh"),
        ("calc", "../mueval/run.sh"),
        ("type", "../mueval/type.sh"),
        ("utf", "../compare_encoding/compare_encoding.sh"),
        ("lhc", "../lhc/lhc_info.sh"),
    ];

    let mut bot = bot::Bot::new(client, scripts);
    let mut stream = bot.stream()?;
    loop {
        tokio::select! {
            _ = shutdown_notify.notified() => bot.quit(None)?,
            message = stream.next() => {
                match message {
                    Some(Ok(message)) => bot.handle_message(&message)?,
                    Some(Err(e)) => {
                        eprintln!("Error: {}", e);
                        break;
                    }
                    None => break,
                }
            }
        }
    }

    Ok(())
}
