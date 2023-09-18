use std::{
    collections::HashMap,
    io::Write,
    net::{TcpListener, TcpStream},
    sync::{Arc, Mutex},
};

use tokio::sync::mpsc::{self, Receiver, Sender};

struct ChannelMessage {
    origin: String,
    message: String,
}

#[tokio::main]
async fn main() {
    let listener = TcpListener::bind("localhost:8080").unwrap();

    let (sender, mut receiver): (Sender<ChannelMessage>, Receiver<ChannelMessage>) =
        mpsc::channel(100);
    let mut _clients: HashMap<String, TcpStream> = HashMap::new();

    let clients = Arc::new(Mutex::new(_clients));
    let clients_clone = clients.clone();

    tokio::spawn(async move {
        loop {
            if let Some(message) = receiver.recv().await {
                let clients_list = clients_clone.lock().unwrap();
                for (client_addr, mut client_stream) in clients_list.iter() {
                    if client_addr.clone() != message.origin {
                        client_stream
                            .write_all(format!("{}: {}", client_addr, message.message).as_bytes())
                            .unwrap();
                    }
                }
            }
        }
    });

    loop {
        let (stream, addr) = listener.accept().unwrap();
        println!("{:?}", stream);
        let stream_clone = stream.try_clone().unwrap();
        println!("{:?}", stream_clone);

        clients
            .lock()
            .unwrap()
            .insert(addr.port().to_string(), stream);

        println!("{:?}", clients);

        stream_clone.set_nonblocking(true).unwrap();
        let tokio_stream = tokio::net::TcpStream::from_std(stream_clone).unwrap();
        let sender_clone = sender.clone();
        tokio::spawn(async move {
            loop {
                tokio_stream.readable().await.unwrap();
                let mut buf = [0; 4096];
                match tokio_stream.try_read(&mut buf) {
                    Ok(0) => continue,
                    Ok(n) => {
                        println!("read {} bytes", n);
                        match sender_clone
                            .send(ChannelMessage {
                                origin: addr.port().to_string(),
                                message: String::from_utf8(buf.to_vec()).unwrap(),
                            })
                            .await
                        {
                            Ok(_) => {}
                            Err(err) => {
                                println!("{err}");
                            }
                        };
                    }
                    Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        println!("would block error");
                        continue;
                    }
                    Err(e) => {
                        panic!("{:?}", e);
                    }
                }
            }
        });
    }
}
