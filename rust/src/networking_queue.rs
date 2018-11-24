/*
 * gRIP
 * Copyright (c) 2018 Alik Aslanyan <cplusplus256@gmail.com>
 *
 *
 *    This program is free software; you can redistribute it and/or modify it
 *    under the terms of the GNU General Public License as published by the
 *    Free Software Foundation; either version 3 of the License, or (at
 *    your option) any later version.
 *
 *    This program is distributed in the hope that it will be useful, but
 *    WITHOUT ANY WARRANTY; without even the implied warranty of
 *    MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU
 *    General Public License for more details.
 *
 *    You should have received a copy of the GNU General Public License
 *    along with this program; if not, write to the Free Software Foundation,
 *    Inc., 59 Temple Place, Suite 330, Boston, MA  02111-1307  USA
 *
 *    In addition, as a special exception, the author gives permission to
 *    link the code of this program with the Half-Life Game Engine ("HL
 *    Engine") and Modified Game Libraries ("MODs") developed by Valve,
 *    L.L.C ("Valve").  You must obey the GNU General Public License in all
 *    respects for all of the code used other than the HL Engine and MODs
 *    from Valve.  If you modify this file, you may extend this exception
 *    to your version of the file, but you are not obligated to do so.  If
 *    you do not wish to do so, delete this exception statement from your
 *    version.
 *
 */

use std::thread;

use futures::prelude::*;
use hyper::rt::*;
use std::sync::Arc;

#[derive(Clone)]
pub enum Type {
    Get,
}

#[derive(Builder)]
pub struct Request {
    id: isize,
    http_type: Type,
    uri: hyper::Uri,
}

struct Response {
    id: isize,
    http_type: Type,
    body: Vec<u8>,
}

enum Command {
    Request(Request),
    Quit,
}

pub struct Queue {
    working_thread: thread::JoinHandle<()>,
    executor: tokio::runtime::TaskExecutor,
    command_sender: futures::sync::mpsc::UnboundedSender<Command>, // TODO:
    response_receiver: crossbeam_channel::Receiver<Response>,      // TODO
}

impl Queue {
    pub fn new() -> Self {
        let mut runtime = tokio::runtime::Runtime::new().unwrap();
        let executor = runtime.executor();
        let (command_sender, command_receiver) = futures::sync::mpsc::unbounded();
        let (response_sender, response_receiver) = crossbeam_channel::unbounded();

        let client = {
            let https = hyper_tls::HttpsConnector::new(4); // TODO: Number of DNS threads?
            crate::client::Client::new(
                hyper::Client::builder()
                    .executor(executor.clone())
                    .build::<_, hyper::Body>(https.unwrap()),
            )
        };

        let working_thread = {
            let executor = executor.clone();
            let response_sender = response_sender.clone();
            thread::spawn(move || {
                let response_sender = response_sender.clone();
                runtime
                    .block_on(lazy(move || {
                        let response_sender = response_sender.clone();
                        command_receiver
                            .take_while(|cmd| {
                                Ok(match cmd {
                                    Command::Quit => false,
                                    _ => true,
                                })
                            }).for_each(move |cmd| {
                                let response_sender = response_sender.clone();
                                match cmd {
                                    Command::Quit => unreachable!(),
                                    Command::Request(req) => executor.spawn(
                                        match req.http_type {
                                            Type::Get => client.get(req.uri.clone()),
                                            _ => unimplemented!(),
                                        }.and_then(move |res| res.into_body().concat2())
                                        .map(move |body| {
                                            use bytes::buf::FromBuf;

                                            response_sender.send(Response {
                                                http_type: req.http_type,
                                                id: req.id,
                                                body: Vec::from_buf(body.into_bytes()),
                                            })
                                        }).map(|_| {})
                                        .map_err(|_| {}), // TODO: Err handling?
                                    ),
                                }

                                Ok(())
                            })
                    })).unwrap();
            })
        };

        Queue {
            working_thread,
            executor,
            command_sender,
            response_receiver,
        }
    }

    pub fn stop(mut self) {
        self.send_command(Command::Quit);
        self.working_thread.join().unwrap(); // TODO: Err handling?
    }

    pub fn send_request(&mut self, request: Request) {
        self.send_command(Command::Request(request));
    }

    fn send_command(&mut self, command: Command) {
        let command_sender = self.command_sender.clone();
        self.executor.spawn(lazy(move || {
            command_sender.send(command).map(|_| {}).map_err(|_| {})
        }));
    }

    pub fn execute_queue(&mut self, limit: usize) -> usize {
        let mut counter = 0;

        while counter <= limit {
            match self.response_receiver.try_recv() {
                Ok(response) => {
                    println!(
                        "Received a body: {:#?}",
                        String::from_utf8_lossy(&response.body[..])
                    );
                    counter += 1;
                }
                Err(_) => break,
            }
        }
        counter
    }
}

mod tests {
    use super::*;
    #[test]
    fn test() {
        let mut queue = Queue::new();

        use std::default::Default;

        queue.send_request(
            RequestBuilder::default()
                .id(1)
                .http_type(Type::Get)
                .uri("https://docs.rs/".parse().unwrap())
                .build()
                .unwrap(),
        );

        loop {
            queue.execute_queue(5);
        }

    }
}
