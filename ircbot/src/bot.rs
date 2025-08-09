use irc::client::prelude::Message;
use irc::{
    client::{ClientStream, prelude::*},
    error,
};
use rand::{Rng, distr::Alphanumeric, rng};
use std::collections::HashMap;
use std::io::Write;
use std::path::Path;
use std::time::Duration;
use std::{env, future};
use tokio::time::{Instant, sleep_until};

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
    magic_key: String,
    scripts: Vec<Script>,
    timers: Vec<Timer>,
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
    pub fn new<T: ClientInterface + 'static>(client: T, scripts: Vec<Script>) -> Self {
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
            magic_key,
            scripts,
            timers: Vec::new(),
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

    pub fn stream(&mut self) -> error::Result<ClientStream> {
        self.client.stream()
    }

    pub fn quit(&mut self, message: Option<String>) -> error::Result<()> {
        self.execute_response(Response::Quit(message), "")
    }

    pub fn handle_message(&mut self, message: &Message) -> error::Result<()> {
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
            _ => Ok(()),
        }
    }

    fn handle_privmsg(
        &mut self,
        message: &str,
        message_data: &MessageMetadata,
    ) -> error::Result<()> {
        let response = self.generate_response(message, message_data);
        self.execute_response(response, &message_data.response_target)
    }

    fn execute_response(&mut self, response: Response, response_target: &str) -> error::Result<()> {
        match response {
            Response::None => Ok(()),
            Response::Quit(msg) => self.client.send(Command::QUIT(Some(
                msg.unwrap_or_else(|| "さようなら".to_string()),
            ))),
            Response::Reply(msg) => self.execute_response(
                Response::Privmsg(response_target.to_string(), msg),
                response_target,
            ),
            Response::Join(channel) => self.client.send(Command::JOIN(channel, None, None)),
            Response::Part(channel) => self.client.send(Command::PART(channel, None)),
            Response::Privmsg(target, msg) => {
                let lines = msg.lines().filter(|line| !line.trim().is_empty());
                // Limit maximum number of lines and line length.
                for msg in lines.take(4) {
                    self.client.send(Command::PRIVMSG(
                        target.to_string(),
                        limit_length(msg, 410).to_string(),
                    ))?;
                }
                Ok(())
            }
        }
    }

    fn generate_response(&mut self, message: &str, message_data: &MessageMetadata) -> Response {
        let message = if let Some(msg) = message.strip_prefix('!') {
            msg
        } else if message_data.target.starts_with('#') {
            // Channel message without ! prefix
            return Response::None;
        } else {
            self.debug_out(&format!("<{}> {}", message_data.sender, message));
            // Private message to the bot works without ! prefix
            message
        };

        if message.starts_with(&self.magic_key) {
            return self.do_special_command(message[self.magic_key.len()..].trim_start());
        }

        let (command, args) = self.parse_message(message);
        if let Some(command_handler) = self.commands.get(command.as_str()) {
            return command_handler(self, &args);
        }

        for script in &self.scripts {
            if *script.name == command {
                let output = self.run_script(
                    &script.path,
                    &args,
                    &message_data.sender,
                    &message_data.response_target,
                    false,
                );
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
                return Response::Reply(output);
            }
        }

        Response::None
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
    }

    pub async fn next_timer(&mut self) -> TimerData {
        if self.timers.is_empty() {
            return future::pending::<TimerData>().await;
        }
        self.timers.sort_by_key(|timer| timer.deadline);
        sleep_until(self.timers[0].deadline).await;
        self.timers.remove(0).data
    }

    pub fn run_timed_command(&mut self, data: TimerData) -> error::Result<()> {
        let output = self.run_script(
            &data.script,
            &data.argument,
            &data.sender,
            &data.response_target,
            false,
        );
        let filtered =
            self.extract_timer_commands(&output, &data.script, &data.sender, &data.response_target);
        self.debug_out(&format!(
            "Executing timed command: {:?}\nResponse: {:?}",
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

    fn run_script(
        &self,
        path: &str,
        argument: &str,
        sender: &str,
        response_target: &str,
        ignore_errors: bool,
    ) -> String {
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

        let working_dir = match Path::new(path).parent() {
            Some(p) => p,
            None => return format!("Invalid script path: {}", path).to_string(),
        };
        let output = std::process::Command::new(path)
            .arg(argument)
            .current_dir(working_dir)
            .envs(
                env.iter()
                    .map(|(k, v)| (k.as_str(), v.as_str()))
                    .collect::<Vec<(&str, &str)>>(),
            )
            .output();

        match output {
            Ok(out) => {
                if out.status.success() {
                    String::from_utf8_lossy(&out.stdout).to_string()
                } else {
                    self.debug_out(&format!(
                        "[{}] Error running {} {}\nstderr: {}\nstdout: {}",
                        out.status,
                        path,
                        argument,
                        String::from_utf8_lossy(&out.stderr),
                        String::from_utf8_lossy(&out.stdout)
                    ));
                    "An error ocurred.".to_string()
                }
            }
            Err(e) => {
                if ignore_errors {
                    String::new()
                } else {
                    self.debug_out(&format!(
                        "Internal error running {} {}: {}",
                        path, argument, e
                    ));
                    "An error occurred.".to_string()
                }
            }
        }
    }
}

fn version_command(_bot: &Bot, _args: &str) -> Response {
    Response::Reply("A very simple bot with 日本語 support.".to_string())
}

fn help_command(bot: &Bot, _args: &str) -> Response {
    let mut command_names: Vec<&str> = bot.commands.keys().copied().collect();
    command_names.sort();
    let mut commands_list: Vec<String> = command_names
        .iter()
        .map(|cmd| format!("!{}", cmd))
        .collect();
    commands_list.sort();
    let commands_list = commands_list.join(", ");
    Response::Reply(format!("Known commands: {}", commands_list))
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
