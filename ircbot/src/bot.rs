use chrono::Local;
use gettextrs::gettext;
use irc::client::prelude::Message;
use irc::{
    client::{ClientStream, prelude::*},
    error,
};
use rand::{Rng, distr::Alphanumeric, rng};
use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{self, BufRead, BufReader, Write};
use std::path::Path;
use std::time::Duration;
use std::{env, future};
use tokio::time::{Instant, sleep_until};

use crate::error::BotError;

const PING_INTERVAL_SECONDS: u64 = 60;

pub trait ClientInterface {
    fn send(&mut self, command: Command) -> error::Result<()>;
    fn stream(&mut self) -> error::Result<ClientStream>;
}

impl ClientInterface for Client {
    fn send(&mut self, command: Command) -> error::Result<()> {
        self::Client::send(self, command)
    }
    fn stream(&mut self) -> error::Result<ClientStream> {
        self::Client::stream(self)
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct TimerData {
    script: String,
    argument: String,
    sender: String,
    response_target: String,
}

struct Timer {
    deadline: Instant,
    data: TimerData,
}

pub struct Bot {
    commands: HashMap<&'static str, CommandFn>,
    client: Box<dyn ClientInterface>,
    main_channel: String,
    main_channel_topic: Option<String>,
    magic_key: String,
    scripts: Vec<Script>,
    timers: Vec<Timer>,
    next_daily_trigger: Instant,
    next_ping_time: Instant,
    failed_pings: u8,
    waiting_for_pong: bool,
}

type CommandFn = fn(&Bot, &str) -> Response;

#[derive(Debug, PartialEq, Clone)]
enum Response {
    None,
    Quit(Option<String>),
    Reply(String),
    Join(String),
    Part(String),
    Privmsg(String, String),
}

struct MessageMetadata {
    sender: String,
    target: String,
    response_target: String,
}

pub struct Script {
    name: String,
    path: String,
    timers_allowed: bool,
}

impl Script {
    pub fn new(name: &str, path: &str) -> Self {
        Script {
            name: name.to_string(),
            path: path.to_string(),
            timers_allowed: false,
        }
    }
    pub fn new_with_timers(name: &str, path: &str) -> Self {
        Script {
            name: name.to_string(),
            path: path.to_string(),
            timers_allowed: true,
        }
    }
}

impl Bot {
    pub fn new<T: ClientInterface + 'static>(
        client: T,
        main_channel: &str,
        scripts: Vec<Script>,
    ) -> Self {
        let magic_key: String = rng()
            .sample_iter(&Alphanumeric)
            .take(8)
            .map(char::from)
            .collect();

        let mut commands: HashMap<&'static str, CommandFn> = HashMap::new();
        commands.insert("version", version_command);
        commands.insert("help", help_command);

        let bot = Bot {
            commands,
            client: Box::new(client),
            main_channel: main_channel.to_string(),
            main_channel_topic: None,
            magic_key,
            scripts,
            timers: Vec::new(),
            next_daily_trigger: next_midnight(),
            next_ping_time: Instant::now() + Duration::from_secs(PING_INTERVAL_SECONDS),
            failed_pings: 0,
            waiting_for_pong: false,
        };
        bot.print_magic_key();
        bot
    }

    fn print_magic_key(&self) {
        print!("Today's magic key for admin commands: {}", self.magic_key);
        std::io::stdout().flush().unwrap();
    }

    fn debug_out(&self, line: &str) {
        // Overwrite magic key.
        println!("\r{}\r{}", " ".repeat(60), line);
        self.print_magic_key();
    }

    fn send(&mut self, command: Command) -> Result<(), BotError> {
        self.client.send(command).map_err(BotError::from)
    }

    pub fn stream(&mut self) -> Result<ClientStream, BotError> {
        self.client.stream().map_err(BotError::from)
    }

    pub fn quit(&mut self, message: Option<String>) -> Result<(), BotError> {
        self.execute_response(Response::Quit(message), "")
    }

    pub fn handle_message(&mut self, message: &Message) -> Result<(), BotError> {
        match message.command {
            Command::PRIVMSG(ref target, ref m) => {
                let sender = match message.source_nickname() {
                    Some(sender) => sender.to_string(),
                    None => return Ok(()),
                };
                let response_target = match message.response_target() {
                    Some(target) => target.to_string(),
                    None => return Ok(()),
                };
                self.handle_privmsg(
                    m,
                    &MessageMetadata {
                        sender,
                        target: target.to_string(),
                        response_target,
                    },
                )
            }
            Command::TOPIC(ref channel, ref topic) => {
                if let Some(topic) = topic {
                    self.handle_topic_change(channel, topic);
                }
                Ok(())
            }
            Command::Response(irc::client::prelude::Response::RPL_TOPIC, ref args) => {
                if args.len() == 3 {
                    self.handle_topic_change(&args[1], &args[2]);
                }
                Ok(())
            }
            Command::PONG(_, _) => {
                self.waiting_for_pong = false;
                Ok(())
            }
            _ => Ok(()),
        }
    }

    fn handle_privmsg(
        &mut self,
        message: &str,
        message_data: &MessageMetadata,
    ) -> Result<(), BotError> {
        let response = self.generate_response(message, message_data)?;
        self.execute_response(response, &message_data.response_target)
    }

    fn handle_topic_change(&mut self, channel: &str, topic: &str) {
        if channel == self.main_channel {
            self.main_channel_topic = Some(topic.to_string());
        }
    }

    fn execute_response(
        &mut self,
        response: Response,
        response_target: &str,
    ) -> Result<(), BotError> {
        match response {
            Response::None => Ok(()),
            Response::Quit(msg) => self.send(Command::QUIT(Some(
                msg.unwrap_or_else(|| "さようなら".to_string()),
            ))),
            Response::Reply(msg) => self.execute_response(
                Response::Privmsg(response_target.to_string(), msg),
                response_target,
            ),
            Response::Join(channel) => self.send(Command::JOIN(channel, None, None)),
            Response::Part(channel) => self.send(Command::PART(channel, None)),
            Response::Privmsg(target, msg) => {
                let lines = msg.lines().filter(|line| !line.trim().is_empty());
                // Limit maximum number of lines and line length.
                for msg in lines.take(4) {
                    self.send(Command::PRIVMSG(
                        target.to_string(),
                        limit_length(msg, 410).to_string(),
                    ))?;
                }
                Ok(())
            }
        }
    }

    fn generate_response(
        &mut self,
        message: &str,
        message_data: &MessageMetadata,
    ) -> Result<Response, BotError> {
        let message = if let Some(msg) = message.strip_prefix('!') {
            msg
        } else if message_data.target.starts_with('#') {
            // Channel message without ! prefix
            return Ok(Response::None);
        } else {
            self.debug_out(&format!("<{}> {}", message_data.sender, message));
            // Private message to the bot works without ! prefix
            message
        };

        if message.starts_with(&self.magic_key) {
            return Ok(self.do_special_command(message[self.magic_key.len()..].trim_start()));
        }

        let (command, args) = self.parse_message(message);
        if let Some(command_handler) = self.commands.get(command.as_str()) {
            return Ok(command_handler(self, &args));
        }

        for script in &self.scripts {
            if *script.name == command {
                let output = match self.run_script(
                    &script.path,
                    &args,
                    &message_data.sender,
                    &message_data.response_target,
                    false,
                ) {
                    Ok(output) => output,
                    Err(e) => {
                        self.debug_out(&format!("run_script: {}", e));
                        return Ok(Response::Reply(gettext("An error occurred.")));
                    }
                };
                let output = if script.timers_allowed {
                    self.extract_timer_commands(
                        &output,
                        &script.path.to_string(),
                        &message_data.sender,
                        &message_data.response_target,
                    )
                } else {
                    output
                };
                return Ok(Response::Reply(output));
            }
        }

        Ok(Response::None)
    }

    fn do_special_command(&self, cmd: &str) -> Response {
        let (command, args) = cmd.split_once(' ').unwrap_or((cmd, ""));
        match command {
            "die" => {
                if !args.is_empty() {
                    Response::Quit(Some(args.to_string()))
                } else {
                    Response::Quit(None)
                }
            }
            "join" => {
                if !args.is_empty() {
                    Response::Join(args.to_string())
                } else {
                    Response::Reply("Missing channel name".to_string())
                }
            }
            "part" => {
                if !args.is_empty() {
                    Response::Part(args.to_string())
                } else {
                    Response::Reply("Missing channel name".to_string())
                }
            }
            "privmsg" => {
                let args_parts: Vec<&str> = args.splitn(2, ' ').collect();
                if args_parts.len() == 2 {
                    Response::Privmsg(args_parts[0].to_string(), args_parts[1].to_string())
                } else {
                    Response::Reply("Usage: privmsg <target> <message>".to_string())
                }
            }
            _ => Response::Reply("Unknown admin command".to_string()),
        }
    }

    fn parse_message(&self, message: &str) -> (String, String) {
        if let Some((cmd, args)) = message.split_once(' ') {
            (cmd.to_string(), args.to_string())
        } else if let Some((cmd, args)) = message.split_once('　') {
            (cmd.to_string(), args.to_string())
        } else {
            (message.to_string(), String::new())
        }
    }

    fn add_timer(
        &mut self,
        delay_seconds: u64,
        script: &str,
        argument: &str,
        sender: &str,
        room: &str,
    ) {
        self.timers.push(Timer {
            deadline: Instant::now() + Duration::from_secs(delay_seconds),
            data: TimerData {
                script: script.to_string(),
                argument: argument.to_string(),
                sender: sender.to_string(),
                response_target: room.to_string(),
            },
        });
        self.timers.sort_by_key(|timer| timer.deadline);
    }

    pub async fn next_timer(&self) {
        if self.timers.is_empty() {
            return future::pending().await;
        }
        sleep_until(self.timers[0].deadline).await;
    }

    pub async fn next_ping(&self) {
        sleep_until(self.next_ping_time).await;
    }

    pub fn send_ping(&mut self) -> Result<(), BotError> {
        if self.waiting_for_pong {
            self.failed_pings += 1;
            if self.failed_pings >= 3 {
                return Err(BotError::ConnectionLost("Ping timeout".to_string()));
            }
        } else {
            self.failed_pings = 0;
        }

        self.waiting_for_pong = true;
        self.next_ping_time = Instant::now() + Duration::from_secs(PING_INTERVAL_SECONDS);
        self.send(Command::PING("".to_string(), None))
    }

    pub fn run_timed_command(&mut self) -> Result<(), BotError> {
        let data = self.timers.remove(0).data;
        let output = match self.run_script(
            &data.script,
            &data.argument,
            &data.sender,
            &data.response_target,
            true,
        ) {
            Ok(output) => output,
            Err(e) => {
                self.debug_out(&format!("Timed command {:?} failed: {}", data, e));
                return Ok(());
            }
        };
        let filtered =
            self.extract_timer_commands(&output, &data.script, &data.sender, &data.response_target);
        self.debug_out(&format!(
            "Executed timed command: {:?}\nResponse: {:?}",
            data, &filtered
        ));
        self.execute_response(Response::Reply(filtered), &data.response_target)
    }

    fn extract_timer_commands(
        &mut self,
        output: &str,
        script: &str,
        sender: &str,
        room: &str,
    ) -> String {
        let mut filtered = vec![];
        for line in output.lines() {
            if line.starts_with("/timer ") {
                let args: Vec<&str> = line.split(' ').collect();
                if args.len() >= 3 {
                    let delay_seconds = args[1].parse().unwrap_or(0);
                    let argument = args[2..].join(" ");
                    self.add_timer(delay_seconds, script, &argument, sender, room);
                }
            } else {
                filtered.push(line);
            }
        }
        filtered.join("\n")
    }

    pub async fn next_background_job(&self) {
        sleep_until(self.next_daily_trigger + Duration::from_secs(1)).await;
    }

    pub fn run_background_job(&mut self) -> Result<(), BotError> {
        self.next_daily_trigger = next_midnight();
        self.daily_jobs()
    }

    fn run_script(
        &self,
        path: &str,
        argument: &str,
        sender: &str,
        response_target: &str,
        ignore_errors: bool,
    ) -> Result<String, BotError> {
        let mut env = env::vars().collect::<HashMap<String, String>>();
        let lang = env
            .get("LANG")
            .unwrap_or(&"en_US.utf8".to_string())
            .to_string();
        env.insert("DMB_SENDER".to_string(), sender.to_string());
        env.insert("DMB_RECEIVER".to_string(), response_target.to_string());
        env.insert("LANGUAGE".to_string(), lang.to_string());
        env.insert("LANG".to_string(), lang.to_string());
        env.insert("LC_ALL".to_string(), lang);
        env.insert("IRC_PLUGIN".to_string(), "1".to_string());

        let working_dir = Path::new(path)
            .parent()
            .ok_or(BotError::InvalidScriptPath(format!(
                "Invalid directory: {}",
                path
            )))?;
        let output = std::process::Command::new(path)
            .arg(argument)
            .current_dir(working_dir)
            .envs(env)
            .output();

        match output {
            Ok(out) => {
                if out.status.success() {
                    Ok(String::from_utf8_lossy(&out.stdout).to_string())
                } else {
                    Err(BotError::ScriptExecutionError(format!(
                        "[{}] Error running {} {}\nstderr: {}\nstdout: {}",
                        out.status,
                        path,
                        argument,
                        String::from_utf8_lossy(&out.stderr),
                        String::from_utf8_lossy(&out.stdout)
                    )))
                }
            }
            Err(e) => {
                if ignore_errors {
                    self.debug_out(&format!(
                        "Internal error ignored running {} {}: {}",
                        path, argument, e
                    ));
                    Ok(String::new())
                } else {
                    match e.kind() {
                        std::io::ErrorKind::NotFound => {
                            Err(BotError::InvalidScriptPath(format!("Not found: {}", path)))
                        }
                        _ => Err(BotError::ScriptExecutionError(format!(
                            "Internal error running {} {}: {}",
                            path, argument, e
                        ))),
                    }
                }
            }
        }
    }

    fn next_word_of_the_day(&self, old_word: &str) -> io::Result<Option<String>> {
        let file_done = "word_of_the_day_done.txt";
        let file_next = "word_of_the_day_next.txt";

        let mut input_lines = BufReader::new(File::open(file_next)?).lines();
        let next_word = if let Some(line) = input_lines.next() {
            line?
        } else {
            return Ok(None);
        };

        let mut temp_file = File::create(format!("{}.tmp", file_next))?;
        for line in input_lines {
            let line = line?;
            writeln!(temp_file, "{}", line)?;
        }
        std::fs::rename(format!("{}.tmp", file_next), file_next)?;

        if !next_word.is_empty() {
            let mut file = OpenOptions::new()
                .create(true)
                .append(true)
                .open(file_done)?;
            writeln!(file, "{}", old_word)?;
        }

        Ok(Some(next_word.trim().to_string()))
    }

    pub fn daily_jobs(&mut self) -> Result<(), BotError> {
        let marker = "Wort des Tages: ";
        if let Some(ref topic) = self.main_channel_topic
            && let Some(pos) = topic.find(marker)
        {
            let prefix = &topic[..pos];
            let old_word_part = &topic[pos + marker.len()..];

            let (old_word, suffix) = if let Some(space_pos) = old_word_part.find(' ') {
                let (word, suf) = old_word_part.split_at(space_pos);
                (word, " ".to_string() + suf)
            } else {
                (old_word_part, String::new())
            };

            let new_word = match self.next_word_of_the_day(old_word) {
                Ok(Some(new_word)) => new_word,
                _ => return Ok(()), // Ignore errors, it's a non-essential feature
            };

            let new_topic = format!("{}{}{}{}", prefix, marker, new_word, suffix);
            self.debug_out(&format!("New topic: [{}]", new_topic));
            self.client
                .send(Command::TOPIC(self.main_channel.clone(), Some(new_topic)))?;
        }
        Ok(())
    }
}

fn version_command(_bot: &Bot, _args: &str) -> Response {
    Response::Reply(gettext("A very simple bot with 日本語 support.").to_string())
}

fn help_command(bot: &Bot, _args: &str) -> Response {
    let mut command_names: Vec<&str> = bot.commands.keys().copied().collect();
    command_names.sort();
    let mut commands_list: Vec<String> = command_names
        .iter()
        .map(|cmd| format!("!{}", cmd))
        .collect();
    for script in &bot.scripts {
        commands_list.push(format!("!{}", script.name));
    }
    commands_list.sort();
    let commands_list = commands_list.join(", ");
    Response::Reply(format!("{}{}", gettext("Known commands: "), commands_list))
}

/// Safely limits the length of a unicode string.
fn limit_length(s: &str, max_bytes: usize) -> &str {
    if s.len() <= max_bytes {
        return s;
    }
    for (limit, _) in s.char_indices().rev() {
        if limit <= max_bytes {
            return &s[..limit];
        }
    }
    ""
}

fn next_midnight() -> Instant {
    let now = Local::now();
    let midnight = now
        .date_naive()
        .succ_opt()
        .unwrap()
        .and_hms_opt(0, 0, 0)
        .unwrap();
    let midnight_with_timezone = midnight.and_local_timezone(*now.offset()).unwrap();
    let duration_until_midnight = midnight_with_timezone.signed_duration_since(now);
    Instant::now() + Duration::from_secs(duration_until_midnight.num_seconds() as u64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_limit_length_ascii() {
        assert_eq!(limit_length("hello", 5), "hello");
        assert_eq!(limit_length("hello world", 5), "hello");
        assert_eq!(limit_length("hello", 10), "hello");
        assert_eq!(limit_length("hello", 0), "");
    }

    #[test]
    fn test_limit_length_japanese() {
        assert_eq!(limit_length("こんにちは", 15), "こんにちは"); // 5 chars * 3 bytes = 15
        assert_eq!(limit_length("こんにちは", 12), "こんにち"); // 4 chars * 3 bytes = 12
        assert_eq!(limit_length("こんにちは", 3), "こ"); // 1 char * 3 bytes = 3
        assert_eq!(limit_length("こんにちは", 2), ""); // Not enough for 1 char
        for i in 1..=16 {
            limit_length("こんにちは", i); // assert no panics
        }
    }

    #[test]
    fn test_limit_length_mixed() {
        assert_eq!(limit_length("hello こんにちは", 10), "hello こ"); // 6 ASCII + 1 Japanese = 9 bytes
    }

    #[test]
    fn test_limit_length_empty_string() {
        assert_eq!(limit_length("", 10), "");
        assert_eq!(limit_length("", 0), "");
    }
}
