use std::io;
use std::io::Read;
use std::net::SocketAddr;
use std::str::from_utf8;
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender, TryRecvError};
use std::time::Duration;
use anyhow::{bail, Result};
use mio::{Events, Interest, Poll, Token};
use mio::net::{TcpListener, TcpStream};
use crate::chunk_viewer::event_handler::State;
use crate::chunk_viewer::task_list::{Task, TaskList};
use crate::chunk_viewer::tools::chunk_debug::ChunkDebugEntry;

enum ServerEvent {
    Entry(ChunkDebugEntry),
    Connected(SocketAddr),
    Disconnected,
    Shutdown,
}

enum ControlMessage {
    Shutdown
}

#[derive(PartialEq, Eq, Copy, Clone)]
pub enum ServerStatus {
    Running,
    Connected(SocketAddr),
    Stopped
}

pub(crate) struct ChunkDebugServer {
    event_receiver: Receiver<ServerEvent>,
    message_sender: Sender<ControlMessage>,

    status: ServerStatus
}
impl ChunkDebugServer {
    pub fn start(port: u16, task_list: &mut TaskList<State>) -> Self {
        let (event_tx, event_rx) = mpsc::channel::<ServerEvent>();
        let (message_tx, message_rx) = mpsc::channel::<ControlMessage>();

        let our_event_tx = event_tx.clone();

        let server_task = Task::start(move || {
            start_chunk_debug_server(port, event_tx, message_rx)
        }, move |result: Result<()>, _: &mut State| {
            match result {
                Ok(_) => {}
                Err(err) => println!("Server Error: {err}")
            }
            match our_event_tx.send(ServerEvent::Shutdown) {
                Ok(_) => {}
                Err(err) => { println!("Error sending shutdown event: {err}") }
            }
        });
        task_list.add_task("Chunk Debug Server", server_task);

        Self {
            event_receiver: event_rx,
            message_sender: message_tx,
            status: ServerStatus::Running
        }
    }

    pub fn poll(&mut self) -> Vec<ChunkDebugEntry> {
        let mut result = Vec::new();

        for event in self.event_receiver.try_iter() {
            match event {
                ServerEvent::Entry(entry) => { result.push(entry) }
                ServerEvent::Connected(addr) => { self.status = ServerStatus::Connected(addr) }
                ServerEvent::Disconnected => { self.status = ServerStatus::Running }
                ServerEvent::Shutdown => { self.status = ServerStatus::Stopped }
            }
        }

        result
    }

    pub fn get_status(&self) -> ServerStatus {
        self.status
    }

    pub fn shutdown(&mut self) -> Result<()>  {
        self.message_sender.send(ControlMessage::Shutdown)?;
        Ok(())
    }
}

struct LazyLines {
    curr_state: String
}
impl LazyLines {
    fn new() -> Self {
        Self {
            curr_state: String::new()
        }
    }

    fn add_new(&mut self, buf: &[u8]) -> Result<Vec<String>> {
        self.curr_state.push_str(from_utf8(buf)?);

        let lines: Vec<_> = self.curr_state.split("\n").map(|s| String::from(s)).collect();
        let mut result: Vec<String> = Vec::new();
        if lines.len() > 1 {
            let last = lines.len() - 1;
            for (i, line) in lines.into_iter().enumerate() {
                if i != last {
                    result.push(line);
                } else {
                    self.curr_state = line;
                }
            }
        }
        Ok(result)
    }
}

fn start_chunk_debug_server(port: u16, event_tx: Sender<ServerEvent>, message_rx: Receiver<ControlMessage>) -> Result<()> {
    let mut poll = Poll::new()?;
    let mut events = Events::with_capacity(128);

    let addr = format!("127.0.0.1:{port}").parse()?;
    let mut listener = TcpListener::bind(addr)?;

    let mut stream: Option<TcpStream> = None;

    const SERVER: Token = Token(0);
    const STREAM: Token = Token(1);

    poll.registry().register(&mut listener, SERVER, Interest::READABLE)?;

    let mut lazy_lines = LazyLines::new();

    println!("Server starting");

    loop {
        match message_rx.try_recv() {
            Ok(ControlMessage::Shutdown) => break,
            Err(TryRecvError::Disconnected) => break,
            Err(TryRecvError::Empty) => {}
        }

        poll.poll(&mut events, Some(Duration::from_millis(100)))?;

        for event in &events {
            match event.token() {
                SERVER => loop {
                    match listener.accept() {
                        Ok((mut connection, address)) => {
                            println!("Connection from: {}", address);
                            if stream.is_some() {
                                println!("Already connected");
                                drop(connection);
                            } else {
                                poll.registry().register(&mut connection, STREAM, Interest::READABLE)?;
                                stream = Some(connection);
                                event_tx.send(ServerEvent::Connected(address))?;
                            }
                        },
                        Err(ref err) if would_block(err) => break,
                        Err(err) => bail!(err)
                    }
                },
                STREAM => {
                    if event.is_readable() {
                        let mut connection_closed = false;
                        if let Some(stream) = &mut stream {
                            loop {
                                let mut buf = [0; 8192];
                                match stream.read(&mut buf) {
                                    Ok(0) => {
                                        connection_closed = true;
                                        break;
                                    },
                                    Ok(bytes) => {
                                        let lines = lazy_lines.add_new(&buf[0..bytes])?;
                                        for line in lines {
                                            let entry: ChunkDebugEntry = line.parse()?;
                                            event_tx.send(ServerEvent::Entry(entry))?;
                                        }
                                    },
                                    Err(ref err) if would_block(err) => break,
                                    Err(ref err) if interrupted(err) => continue,
                                    Err(err) => bail!(err)
                                };
                            }
                        }

                        if connection_closed {
                            println!("Connection closed");
                            poll.registry().deregister(stream.as_mut().unwrap())?;
                            stream = None;
                            event_tx.send(ServerEvent::Disconnected)?;
                        }
                    }
                }
                _ => {}
            }
        }
    }

    println!("Server stopping");

    Ok(())
}

fn would_block(err: &io::Error) -> bool {
    err.kind() == io::ErrorKind::WouldBlock
}

fn interrupted(err: &io::Error) -> bool {
    err.kind() == io::ErrorKind::Interrupted
}