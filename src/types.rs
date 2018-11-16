
pub type ClientName = String;
type ChatMessage = String;

#[derive(Debug, Serialize, Deserialize)]
pub enum ClientMessage {
    Join(ClientName),
    Message(ChatMessage),
    Nickname(ClientName),
    Quit,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ServerMessage {
    Joined(ClientName),
    Message(ChatMessage, ClientName),
    ServerText(ChatMessage),
    Quit(ClientName),
}
