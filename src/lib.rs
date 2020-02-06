use tokio::net::TcpStream;
use tokio::io::{AsyncWriteExt, AsyncReadExt};

use std::io;
use std::net::SocketAddr;

use ostrich_core::{RawMessage, Command, PCK_SIZE};

pub struct Client {
    pub username: Option<String>,
    password: Option<String>,

    pub addr: Option<SocketAddr>,
    
    socket : Option<TcpStream>,
}

impl Client {

    pub fn new(username: Option<String>, 
               password: Option<String>, 
               addr: Option<SocketAddr>) -> Client {

        Client { username, password, addr, socket : None }
    }

    pub fn get_name(&self) -> Result<&str, io::Error> {
        match &self.username {
            Some(name) => Ok(name),
            None => Err(io::Error::new(io::ErrorKind::NotFound, 
                                       "Username not found")),
        }
    }

    fn get_passw(&self) -> Result<&str, io::Error> {
        match &self.password {
            Some(passw) => Ok(passw),
            None => Err(io::Error::new(io::ErrorKind::NotFound, 
                                       "Password not found")),
        }
    }

    pub fn get_addr(&self) -> Result<&SocketAddr, io::Error> {
        match &self.addr {
            Some(addr) => Ok(addr),
            None => Err(io::Error::new(io::ErrorKind::NotFound, 
                                       "Server address not found")),
        }
    }

    fn get_mut_socket(&mut self) -> Result<&mut TcpStream, io::Error> {
        match &mut self.socket {
            Some(socket) => Ok(socket),
            None => Err(io::Error::new(io::ErrorKind::NotConnected, 
                                       "Not connected to any server")),
        }
    }

    pub fn is_connected(&self) -> bool {
        self.socket.is_some()
    }


    pub fn set_password(&mut self, new_pass: String) {
        self.password = Some(new_pass);
    } 

    pub async fn log_in(&mut self) -> Result<(), io::Error> {
        // Get server addr
        let addr = self.get_addr()?;
        // Get username and password
        let username = self.get_name()?.to_string();
        let password = self.get_passw()?.to_string();

        // Connect to the server
        let mut socket = TcpStream::connect(&addr).await?; 

        // Request USR login
        socket.write(
                &RawMessage::to_raw(
                    &Command::Usr(username, password)
                )?
        ).await?;

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
            Err(err) => return Err(io::Error::new(io::ErrorKind::ConnectionRefused,
                                       format!("Cannot get a response from the server: {}", err))),
        }

        // Save the spcket for later use 
        self.socket = Some(socket);
        Ok(())
    }

    pub async fn get(&mut self) -> Result<Command, io::Error> {
        // Get socket
        let socket = self.get_mut_socket()?;

        // Send GET Command
        socket.write(&RawMessage::to_raw(&Command::Get)?).await?;

        // Read output
        let mut buff = [0u8;PCK_SIZE];
        let _n = socket.read(&mut buff).await?;
        
        RawMessage::from_raw(&buff)
    }
    
    pub async fn send(&mut self, target: String, message: String) -> Result<(), io::Error> {
        // Get username and socket
        let sender = match &self.username {
            Some(name) => name.to_string(), 
            None => panic!(),
        };

        let mut socket = match &mut self.socket {
            Some(s) => Box::new(s),
            None => panic!(),
        };

        // Send the command
        socket.write(
            &RawMessage::to_raw(
                &Command::Msg(sender, target, message)
            )?
        ).await?;

        Ok(())
    }
}
