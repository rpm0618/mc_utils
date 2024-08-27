use std::io::{BufRead, BufReader};
use std::net::TcpListener;
use anyhow::Result;


fn main() -> Result<()> {
    let listener = TcpListener::bind("127.0.0.1:20000")?;
    for stream in listener.incoming() {
        println!("Connection Made");
        let mut stream = stream?;
        
        let buf_reader = BufReader::new(&mut stream);
        for line in buf_reader.lines() {
            let line = line?;
            println!("{line}");
        }
    }
    Ok(())
}
