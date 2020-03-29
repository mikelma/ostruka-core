use tokio::net::TcpStream;
use tokio::io::{AsyncWriteExt, AsyncReadExt, AsyncRead};
use tokio::stream::Stream;
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};

use core::task::{Poll, Context};
use core::pin::Pin;
use std::io;
use std::net::SocketAddr;

use ostrich_core::{RawMessage, Command, PCK_SIZE};

pub type Tx = UnboundedSender<Command>;
pub type Rx = UnboundedReceiver<Command>;

pub struct Client {
    pub username: String,
    socket : TcpStream,
    rx : Rx,
}

impl Client {

    pub async fn log_in(name: &str, 
                        password: &str, 
                        addr: SocketAddr) -> Result<(Client, Tx), io::Error> {

        // Connect to the server
        let mut socket = TcpStream::connect(&addr).await?; 

        // Request USR login
        socket.write_all(
                &RawMessage::to_raw(
                    &Command::Usr(
                        name.to_string(), 
                        password.to_string()))?).await?;

        // Read the server's response
        let mut data = [0;PCK_SIZE];
        let _n = socket.read(&mut data).await?;
        
        match RawMessage::from_raw(&data) {
            Ok(comm) => {
                match comm {
                    Command::Ok => (),
                    Command::Err(err) => {
                        return Err(io::Error::new(
                                io::ErrorKind::PermissionDenied, 
                                err.to_string()))
                    },
                    _ => return Err(io::Error::new(
                            io::ErrorKind::InvalidData, 
                            "Unknown command")),
                }
            },
            Err(err) => return Err(
                io::Error::new(io::ErrorKind::ConnectionRefused,
                               format!(
                                   "Cannot get a response from the server: {}",
                                   err))),
        }

        let (tx, rx) = mpsc::unbounded_channel();

        Ok((Client { 
            username: name.to_string(), 
            socket,
            rx
        }, tx))
    }

    pub async fn send(&mut self, 
                      target: String, 
                      message: String) -> Result<(), io::Error> {
        // Send the command
        self.socket.write_all(
            &RawMessage::to_raw(
                &Command::Msg(self.username.clone(), target, message)
            )?
        ).await?;

        Ok(())
    }

    pub async fn send_cmd(&mut self, 
                      command: &Command) -> Result<(), io::Error> {
        // Send the command
        self.socket.write_all(
            &RawMessage::to_raw(command)?).await?;
        Ok(())
    }
}
/// Enum to distinguish different types of messages that the client needs to handle.
pub enum Message {
    /// The client has something to send to the server
    ToSend(Command),    
    /// The client has received a message from the server
    Received(Command),  
}

impl Stream for Client {

    type Item = Result<Message, io::Error>;

    fn poll_next(mut self: Pin<&mut Self>, 
                 cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        
        // Check for messages to send
        if let Poll::Ready(Some(v)) = Pin::new(&mut self.rx).poll_next(cx) {
            return Poll::Ready(Some(Ok(Message::ToSend(v))));
        }

        // Check if we have received something from the server
        let mut data = [0u8; PCK_SIZE];
        let n = match Pin::new(&mut self.socket).poll_read(cx, &mut data) {
            Poll::Ready(Ok(n)) => n,
            Poll::Ready(Err(err)) => return Poll::Ready(Some(Err(err))),
            Poll::Pending => return Poll::Pending,
        };
        
        if n > 0 {
            let command = RawMessage::from_raw(&data)?;
            return Poll::Ready(Some(Ok(Message::Received(command))));

        } else {
            return Poll::Ready(None);
        }
    }
}
