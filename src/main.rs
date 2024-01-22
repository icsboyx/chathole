mod defs;

mod terminal_ansi;
use colored::Colorize;
use terminal_ansi::*;

use std::io::{self, Read, Write};

use std::{
    net::TcpListener,
    thread::{self},
};

macro_rules! spawn_thread {
    ($name:expr, $thread:block) => {
        thread::Builder::new()
            .name($name.to_string())
            .spawn(move || -> Result<()> { $thread })?
    };
}
use anyhow::Result;
use defs::*;
use terminal_ansi::{formatted_terminal, update_prompt};

fn main() -> Result<()> {
    let server_engine = ServerEngine::new();
    let mut threads_handle = Vec::new();

    let clone_server_engine = server_engine.clone();
    let handler = spawn_thread!("main_server", { main_server(clone_server_engine) });
    threads_handle.push(handler);

    let clone_server_engine = server_engine.clone();
    let handler = spawn_thread!("handle_service_bus", {
        handle_service_bus(clone_server_engine.clone())
    });
    threads_handle.push(handler);

    for handle in threads_handle {
        let join = handle.join();
        println!("Thread joined: {:?}", join)
    }
    Ok(())
}

fn main_server(server: ServerEngine) -> Result<()> {
    let clients = server.clients;
    let channels = server.channels;
    let service_bus = server.service_bus;
    let broadcast_channel = Channel::new(0, "broadcast".to_string()).as_arc_mut();
    channels.lock().unwrap().add_channel(broadcast_channel)?;

    loop {
        let tcp_server = TcpListener::bind("0.0.0.0:2121")?;
        for stream in tcp_server.incoming() {
            let incoming_stream = stream?;
            incoming_stream.set_nonblocking(true)?;
            let id = clients.lock().unwrap().list.len();
            let nick = format!("Anonymous-{}", id);
            let mut client =
                Client::new(id, nick, 20, channels.lock().unwrap().get_default_channel());
            client.terminal.header = "Rust Coded IcsBoyX ChatHole server".to_string();
            let client = client.as_arc_mut();

            let stream = ClientStream::new(incoming_stream, client.clone(), service_bus.clone());
            clients.lock().unwrap().list.push(client.clone());

            channels.lock().unwrap().list[0]
                .lock()
                .unwrap()
                .add_subscriber(client.lock().unwrap().rx.clone())?;
            spawn_thread!(format!("client-{id}"), { handle_client(stream) });
        }
    }
}

fn handle_client(mut session: ClientStream) -> Result<()> {
    let mut full_buffer = Vec::new();
    let _ = session.stream.write(init_terminal().as_bytes())?;
    let _ = session
        .stream
        .write(formatted_terminal(&mut session.client.lock().unwrap().terminal).as_bytes())?;

    loop {
        let mut buffer = [0; 1024];
        let mut payload: String;
        match session.stream.read(&mut buffer) {
            Ok(0) => {
                session
                    .client
                    .lock()
                    .unwrap()
                    .terminal
                    .add_message(ChatMessage::new(
                        "SERVER".blue().bold().to_string(),
                        "See you later alligator!".to_string(),
                    ));
                let _ = session.stream.write(restore_terminal().as_bytes())?;
                session.shutdown()?;
                break;
            }
            Ok(n) => {
                println!("read {:?} bytes", &buffer[..n]);
                if is_ctrl_c(&buffer[..n]) {
                    session
                        .client
                        .lock()
                        .unwrap()
                        .terminal
                        .add_message(ChatMessage::new(
                            "SERVER".blue().bold().to_string(),
                            "See you later alligator!".to_string(),
                        ));
                    let _ = session.stream.write(restore_terminal().as_bytes())?;
                    session.shutdown()?;
                    break;
                }
                if Some(&b'\n') == buffer.get(n - 1) {
                    full_buffer.extend_from_slice(&buffer[..n]);
                    payload = String::from_utf8_lossy(&full_buffer).to_string();
                    payload = payload
                        .trim_end_matches('\n')
                        .trim_end_matches('\r')
                        .to_string();
                    full_buffer.clear();
                } else {
                    full_buffer.extend_from_slice(&buffer[..n]);
                    continue;
                }

                if payload.is_empty() || payload == "\r" {
                    let _ = session.stream.write(
                        update_prompt(&mut session.client.lock().unwrap().terminal).as_bytes(),
                    )?;
                    continue;
                }

                if payload.starts_with('/') {
                    let id = session.client.lock().unwrap().id;
                    session
                        .service_bus
                        .lock()
                        .unwrap()
                        .push_back(CmdMessage::new(id, payload.clone()))?;
                    thread::sleep(std::time::Duration::from_millis(100));
                    let _ = session.stream.write(
                        update_prompt(&mut session.client.lock().unwrap().terminal).as_bytes(),
                    )?;
                    continue;
                }
                let _ = session.stream.write(
                    update_prompt(&mut session.client.lock().unwrap().terminal).as_bytes(),
                )?;

                // ########################################################## //
                let mut receivers = session
                    .client
                    .lock()
                    .unwrap()
                    .channel
                    .lock()
                    .unwrap()
                    .get_all_subscribers();

                for receiver in receivers.iter_mut() {
                    let nick = session.client.lock().unwrap().nick.clone();
                    receiver.push_back(ChatMessage::new(nick, payload.clone()))?;
                }
            }
            Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {}
            Err(e) => {
                println!("Error: {}", e);
                break;
            }
        }

        while !session.client.lock().unwrap().rx.is_empty() {
            let message = session.client.lock().unwrap().rx.pop_front().unwrap();
            session.client.lock().unwrap().terminal.add_message(message);
            let _ = session.stream.write(
                formatted_terminal(&mut session.client.lock().unwrap().terminal).as_bytes(),
            )?;
        }
        thread::sleep(std::time::Duration::from_millis(10));
    }

    Ok(())
}

// ############################################################################################# //
pub fn is_ctrl_c(payload: &[u8]) -> bool {
    let target_sequence: Vec<&[u8]> = vec![&[255, 244, 255, 253, 6], &[3]];
    if target_sequence.contains(&payload) {
        return true;
    }
    false
}

pub fn handle_service_bus(server_engine: ServerEngine) -> Result<()> {
    let mut channels_qt = server_engine.channels.lock().unwrap().len();
    let mut clients_qt = server_engine.clients.lock().unwrap().len();

    loop {
        if server_engine.channels.lock().unwrap().len() != channels_qt {
            channels_qt = server_engine.channels.lock().unwrap().len();
            println!("Channels: {}", channels_qt);
            println!(
                "List of channels: {:?}",
                server_engine.channels.lock().unwrap().list
            );
        }
        if server_engine.clients.lock().unwrap().len() != clients_qt {
            clients_qt = server_engine.clients.lock().unwrap().len();
            println!("Clients: {}", clients_qt);
        }
        while server_engine.service_bus.lock().unwrap().len() > 0 {
            let message = server_engine
                .service_bus
                .lock()
                .unwrap()
                .pop_front()
                .unwrap();
            let id = message.id;

            let command = message.payload;
            let command = command.trim_start_matches('/');
            let command = command.splitn(3, ' ').collect::<Vec<&str>>();
            let client = server_engine.clients.lock().unwrap().get_client(id);
            match command[0].to_lowercase().as_str() {
                "nick" => {
                    if command.len() < 2 {
                        client.lock().unwrap().rx.push_back(ChatMessage::new(
                            "SERVER".blue().bold().to_string(),
                            format!("Command Error: {} is required", "/nick <nick>".yellow()),
                        ))?;
                        continue;
                    }
                    client.lock().unwrap().nick = command[1].to_string();
                    client.lock().unwrap().rx.push_back(ChatMessage::new(
                        "SERVER".blue().bold().to_string(),
                        format!(
                            "Command Success: Nickname changed to {}",
                            command[1].yellow()
                        ),
                    ))?;
                }
                "join" => {
                    if command.len() < 2 {
                        client.lock().unwrap().rx.push_back(ChatMessage::new(
                            "SERVER".blue().bold().to_string(),
                            format!("Command Error: {} is required", "/join <channel>".yellow()),
                        ))?;
                        continue;
                    }

                    let new_channel = server_engine
                        .channels
                        .lock()
                        .unwrap()
                        .clone()
                        .get_channel(command[1].to_string())
                        .clone();

                    if let Some(channel) = new_channel {
                        client.lock().unwrap().unsubscribe_from_channel()?;
                        client.lock().unwrap().channel = channel.clone();
                        client.lock().unwrap().subscribe_to_channel()?;

                        client.lock().unwrap().rx.push_back(ChatMessage::new(
                            "SERVER".blue().bold().to_string(),
                            format!("Command Success: Joined {}", command[1].yellow()),
                        ))?;
                    } else {
                        let new_channel = Channel::new(
                            server_engine.channels.lock().unwrap().len(),
                            command[1].to_string(),
                        )
                        .as_arc_mut();
                        server_engine
                            .channels
                            .lock()
                            .unwrap()
                            .add_channel(new_channel.clone())?;
                        client.lock().unwrap().unsubscribe_from_channel()?;
                        client.lock().unwrap().channel = new_channel.clone();
                        client.lock().unwrap().subscribe_to_channel()?;

                        client.lock().unwrap().rx.push_back(ChatMessage::new(
                            "SERVER".blue().bold().to_string(),
                            format!("Command Success: Joined {}", command[1].yellow()),
                        ))?;
                        continue;
                    }
                }
                "list" => {
                    let channels = server_engine.channels.lock().unwrap().list.clone();
                    let channels = channels.iter().cloned();

                    for channel in channels {
                        let name = channel.lock().unwrap().name.clone();
                        let subscribers = channel.lock().unwrap().subscribers.len();
                        client.lock().unwrap().rx.push_back(ChatMessage::new(
                            "SERVER".blue().bold().to_string(),
                            format!(
                                "channel: {} users: {}",
                                name.yellow(),
                                subscribers.to_string().yellow()
                            ),
                        ))?;
                    }
                }
                "help" => {
                    let commands = [
                        "/nick <nick>".yellow(),
                        "/join <channel>".yellow(),
                        "/list".yellow(),
                        "/help".yellow(),
                    ];

                    for command in commands.iter() {
                        client.lock().unwrap().rx.push_back(ChatMessage::new(
                            "SERVER".blue().bold().to_string(),
                            command.to_string(),
                        ))?;
                    }
                }
                _ => {
                    client.lock().unwrap().rx.push_back(ChatMessage::new(
                        "SERVER".blue().bold().to_string(),
                        format!(
                            "Command Error: {} is not a valid command",
                            command[0].yellow()
                        ),
                    ))?;
                }
            }
        }
        thread::sleep(std::time::Duration::from_millis(10));
    }
}
// fn parse_commands(command: impl AsRef<str>, client: &mut Client) -> String {
//     let payload = command.as_ref().to_string();
//     let command = payload.clone();
//     let command = command.trim_start_matches('/');
//     let command = command.splitn(3, ' ').collect::<Vec<&str>>();
//     match command[0].to_lowercase().as_str() {
//         "nick" => {
//             if command.len() < 2 {
//                 return format!("Command Error: {} is required", "/nick <nick>".yellow());
//             }
//             client.nick = command[1].to_string();
//             format!(
//                 "Command Success: Nickname changed to {}",
//                 command[1].yellow()
//             )
//         }
//         _ => format!(
//             "Command Error: {} is not a valid command",
//             command[0].yellow()
//         ),
//     }
// }
