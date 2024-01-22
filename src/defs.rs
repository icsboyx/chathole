#![allow(dead_code)]
use anyhow::*;

use std::{
    clone,
    collections::VecDeque,
    net::TcpStream,
    ops::Deref,
    sync::{Arc, Mutex, RwLock},
};

enum Commands {
    Nick,
    Join,
}

#[derive(Debug, Clone)]
pub struct ServerEngine {
    pub clients: ArcMut<ClientList>,
    pub channels: ArcMut<ChannelList>,
    pub service_bus: ArcMut<MessageBus<CmdMessage>>,
}
impl ServerEngine {
    pub fn new() -> Self {
        ServerEngine {
            clients: ArcMut::new(ClientList { list: Vec::new() }),
            channels: ArcMut::new(ChannelList::new()),
            service_bus: ArcMut::new(MessageBus::new()),
        }
    }
    pub fn as_arc_mut(&self) -> ArcMut<Self> {
        ArcMut::new(self.clone())
    }
}
impl Default for ServerEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct ClientList {
    pub list: Vec<ArcMut<Client>>,
}
impl ClientList {
    pub fn len(&self) -> usize {
        self.list.len()
    }
    pub fn get_client(&self, id: usize) -> ArcMut<Client> {
        self.list[id].clone()
    }
}

#[derive(Debug, Clone)]
pub struct ChannelList {
    pub list: Vec<ArcMut<Channel>>,
}
impl ChannelList {
    pub fn new() -> Self {
        ChannelList { list: Vec::new() }
    }
    pub fn add_channel(&mut self, channel: ArcMut<Channel>) -> Result<()> {
        self.list.push(channel);
        Ok(())
    }
    pub fn remove_channel(&mut self, client: ArcMut<Channel>) -> Result<()> {
        let index = self.list.iter().position(|x| *x == client).unwrap();
        self.list.remove(index);
        Ok(())
    }
    pub fn len(&self) -> usize {
        self.list.len()
    }
    pub fn as_arc_mut(&self) -> ArcMut<Self> {
        ArcMut::new(self.clone())
    }
    pub fn get_default_channel(&self) -> ArcMut<Channel> {
        self.list[0].clone()
    }
    pub fn get_channel(self, name: String) -> Option<ArcMut<Channel>> {
        let channel = self
            .list
            .iter()
            .cloned()
            .find(|x| x.lock().unwrap().name == name);

        channel
    }
}
impl Default for ChannelList {
    fn default() -> Self {
        Self::new()
    }
}
#[derive(Debug, Clone)]
pub struct Channel {
    pub id: usize,
    pub name: String,
    pub subscribers: Vec<MessageBus<ChatMessage>>,
}
impl Channel {
    pub fn new(id: usize, name: String) -> Self {
        Channel {
            id,
            name,
            subscribers: Vec::new(),
        }
    }
    pub fn add_subscriber(&mut self, subscriber: MessageBus<ChatMessage>) -> Result<()> {
        self.subscribers.push(subscriber);
        Ok(())
    }
    pub fn remove_subscriber(&mut self, subscriber: MessageBus<ChatMessage>) -> Result<()> {
        let index = self
            .subscribers
            .iter()
            .position(|x| *x == subscriber)
            .unwrap();
        self.subscribers.remove(index);
        Ok(())
    }
    pub fn as_arc_mut(&self) -> ArcMut<Self> {
        ArcMut::new(self.clone())
    }
    pub fn get_all_subscribers(&self) -> Vec<MessageBus<ChatMessage>> {
        self.subscribers.clone()
    }
}

pub struct ClientStream {
    pub stream: TcpStream,
    pub client: ArcMut<Client>,
    pub service_bus: ArcMut<MessageBus<CmdMessage>>,
}
impl ClientStream {
    pub fn new(
        stream: TcpStream,
        client: ArcMut<Client>,
        service_bus: ArcMut<MessageBus<CmdMessage>>,
    ) -> Self {
        ClientStream {
            stream,
            client,
            service_bus,
        }
    }
    pub fn shutdown(&mut self) -> Result<()> {
        self.stream.shutdown(std::net::Shutdown::Both)?;
        Ok(())
    }
}
impl Clone for ClientStream {
    fn clone(&self) -> Self {
        ClientStream {
            stream: self.stream.try_clone().unwrap(),
            client: self.client.clone(),
            service_bus: self.service_bus.clone(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct MessageBus<T> {
    pub queue: Arc<RwLock<VecDeque<T>>>,
}
impl<T: Clone> MessageBus<T> {
    pub fn new() -> Self {
        MessageBus {
            queue: Arc::new(RwLock::new(VecDeque::new())),
        }
    }
    pub fn push_back(&mut self, msg: T) -> Result<()> {
        let mut queue = self.queue.write().unwrap();
        queue.push_back(msg.clone());
        Ok(())
    }
    pub fn pop_front(&mut self) -> Option<T> {
        let mut queue = self.queue.write().unwrap();
        queue.pop_front()
    }
    pub fn len(&self) -> usize {
        let queue = self.queue.read().unwrap();
        queue.len()
    }
    pub fn is_empty(&self) -> bool {
        let queue = self.queue.read().unwrap();
        queue.is_empty()
    }
}
impl<T: Clone> Default for MessageBus<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Clone> PartialEq for MessageBus<T> {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.queue, &other.queue)
    }
}

#[derive(Debug, Clone)]
pub struct ArcMut<T>(pub Arc<Mutex<T>>);

impl<T> ArcMut<T> {
    pub fn new(data: T) -> Self {
        ArcMut(Arc::new(Mutex::new(data)))
    }
}
impl<T> Deref for ArcMut<T> {
    type Target = Arc<Mutex<T>>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl<T> PartialEq for ArcMut<T> {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

#[derive(Debug, Clone)]
pub struct Client {
    pub id: usize,
    pub nick: String,
    pub rx: MessageBus<ChatMessage>,
    pub terminal: Terminal,
    pub channel: ArcMut<Channel>,
}
impl Client {
    pub fn new(id: usize, nick: String, terminal_rows: usize, channel: ArcMut<Channel>) -> Self {
        let mut inner_self = Client {
            id,
            nick,
            rx: MessageBus::new(),
            terminal: Terminal::new(terminal_rows),
            channel: channel.clone(),
        };
        inner_self
            .terminal
            .set_prompt(channel.lock().unwrap().name.clone());
        inner_self
    }
    pub fn subscribe_to_channel(&mut self) -> Result<()> {
        self.channel
            .lock()
            .unwrap()
            .add_subscriber(self.rx.clone())?;
        self.terminal
            .set_prompt(self.channel.lock().unwrap().name.clone());
        Ok(())
    }
    pub fn unsubscribe_from_channel(&mut self) -> Result<()> {
        self.channel
            .lock()
            .unwrap()
            .remove_subscriber(self.rx.clone())?;
        Ok(())
    }
    pub fn as_arc_mut(&self) -> ArcMut<Self> {
        ArcMut::new(self.clone())
    }
}

#[derive(Debug, Clone)]
pub struct Terminal {
    pub start_cmd: String,
    pub header: String,
    pub chat: Chat,
    pub prompt: String,
    pub end_cmd: String,
}
impl Terminal {
    pub fn new(chat_lines: usize) -> Self {
        Terminal {
            start_cmd: "".to_string(),
            header: "".to_string(),
            chat: Chat::new(chat_lines),
            prompt: "".to_string(),
            end_cmd: "".to_string(),
        }
    }
    pub fn get_header(&self) -> String {
        self.header.clone()
    }
    pub fn get_chat(&self) -> String {
        self.chat.get_chat()
    }
    pub fn get_prompt(&self) -> String {
        self.prompt.clone()
    }
    pub fn get_terminal_lines(&self) -> usize {
        self.chat.number_of_lines()
    }
    pub fn add_message(&mut self, msg: ChatMessage) {
        self.chat.add_text(msg);
    }
    pub fn as_arc_mut(&self) -> ArcMut<Self> {
        ArcMut::new(self.clone())
    }
    pub fn set_prompt(&mut self, prompt: String) {
        self.prompt = prompt.to_uppercase() + ": ";
    }
}

#[derive(Debug, Clone)]
pub struct Chat {
    chat: VecDeque<String>,
    number_of_lines: usize,
}
impl Chat {
    pub fn new(number_of_lines: usize) -> Self {
        Chat {
            chat: VecDeque::new(),
            number_of_lines,
        }
    }
    pub fn add_text(&mut self, msg: ChatMessage) {
        let msg_text = msg.payload;
        let nick = format!("[{}]: ", msg.nick);
        let padding = nick.len();
        let payload = msg_text
            .chars()
            .collect::<Vec<char>>()
            .chunks(60)
            .enumerate()
            .map(|(i, chunks)| {
                let padded_chunk = if i == 0 {
                    nick.clone() + &chunks.iter().collect::<String>()
                } else {
                    " ".repeat(padding) + &chunks.iter().collect::<String>()
                };
                padded_chunk + "\r\n"
            })
            .collect::<Vec<String>>();
        if self.chat.len() >= self.number_of_lines {
            self.chat.drain(..payload.len());
            self.chat.extend(payload);
            return;
        }
        self.chat.extend(payload);
    }

    pub fn get_chat(&self) -> String {
        let mut chat = String::new();
        for msg in self.chat.iter() {
            chat.push_str(msg);
        }
        chat
    }
    pub fn len(&self) -> usize {
        self.chat.len()
    }
    pub fn number_of_lines(&self) -> usize {
        self.number_of_lines
    }
}

impl PartialEq for Chat {
    fn eq(&self, other: &Self) -> bool {
        self.chat == other.chat
    }
}
#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub nick: String,
    pub payload: String,
}
impl ChatMessage {
    pub fn new(nick: String, msg: String) -> Self {
        ChatMessage { nick, payload: msg }
    }
}

#[derive(Debug, Clone)]
pub struct CmdMessage {
    pub id: usize,
    pub payload: String,
}
impl CmdMessage {
    pub fn new(id: usize, msg: String) -> Self {
        CmdMessage { id, payload: msg }
    }
}
