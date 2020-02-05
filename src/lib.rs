use tokio::net::TcpStream;
use tokio::io::AsyncWriteExt;

use std::io;
use std::net::SocketAddr;
// use std::io::{Write, Read};
// use std::net::{TcpStream, SocketAddr};

use crate::message::RawMessage;

use crate::BUFF_LEN;

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
        let username = self.get_name()?;
        let password = self.get_passw()?;

        // Connect to the server
        let mut socket = TcpStream::connect(&addr).await?; 

        // Request USR login
        socket.write_all(
            format!("USR~{}~{}", username, password).as_bytes())?;
        socket.flush()?;

        // Write the server's response
        let mut data = [0;BUFF_LEN];
        let n = socket.read(&mut data).unwrap();
        
        let data = String::from_utf8_lossy(&data[0..n]);
        
        let mut data = data.split('~');

        match data.next() {
            Some(comm) => {
                match comm {
                    "OK" => (),
                    "ERR" => {
                        let mesg = match data.next() {
                            Some(m) => m,
                            None => "Unknown error",
                        };
                        return Err(io::Error::new(io::ErrorKind::PermissionDenied, mesg))
                    },
                    _ => return Err(io::Error::new(io::ErrorKind::InvalidData, "Unknown command")),
                }
            },
            None => return Err(io::Error::new(io::ErrorKind::ConnectionRefused,
                                       "Cannot get a response from the server")),
        }

        // Save the spcket for later use 
        self.socket = Some(socket);
        Ok(())
    }

    pub fn get(&mut self) -> Result<Option<RawMessage>, io::Error> {
        // Get socket
        let socket = self.get_mut_socket()?;

        // Send GET Command
        socket.write("GET".as_bytes())?;
        socket.flush()?;

        // Read output
        let mut buff = [0u8;BUFF_LEN];
        let n = socket.read(&mut buff)?;
        
        // Check if there is any data to get
        if buff[..n].starts_with(b"END") {
            Ok(None)

        } else {
            // Parse the message
            let message = RawMessage::from_raw(&buff, n)?; 

            // Check if the receiver field is correct
            if message.receiver != self.get_name()? {
                return Err(io::Error::new(
                           io::ErrorKind::InvalidData, 
                           "Invalid message format, unable to get correct receiver"));
            }
             
            Ok(Some(message)) 
        }
    }
    
    // NOTE: Use RawMessage instead of strings
    pub fn send(&mut self, target: &str, message: &str) -> Result<(), io::Error> {
        // Get username and socket
        let sender = match &self.username {
            Some(name) => name, 
            None => panic!(),
        };

        let mut socket = match &mut self.socket {
            Some(s) => Box::new(s),
            None => panic!(),
        };

        // Format the command
        let command = format!("MSG~{}~{}~{}", sender, target, message);
        
        // Send the command
        socket.write(command.as_bytes())?;
        socket.flush()?;

        Ok(())
    }
}
