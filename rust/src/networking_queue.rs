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
use std::time::{Duration, Instant};
use std::mem;

#[derive(Clone, Debug)]
pub enum RequestType {
    Get, // TODO: More types?
}

#[derive(Builder, Constructor, Clone, Debug)]
pub struct Request {
    id: isize,
    http_type: RequestType,
    uri: hyper::Uri,
}

#[derive(Constructor, Builder)]
pub struct Response {
    base_request: Request,
    body: Vec<u8>,
}

enum InputCommand {
    Request(Request, Box<Fn(Response) + Sync + Send>),
    Quit,
}

#[derive(Constructor)]
struct OutputCommand {
    response: Response,
    callback: Box<Fn(Response) + Sync + Send>,
}

pub struct Queue {
    working_thread: Option<thread::JoinHandle<()>>,
    executor: tokio::runtime::TaskExecutor,
    input_command_sender: futures::sync::mpsc::UnboundedSender<InputCommand>,
    response_receiver: crossbeam_channel::Receiver<OutputCommand>,
}

impl Drop for Queue {
    fn drop(&mut self) {
        self.stop();
    }
}


impl Queue {
    pub fn new(number_of_dns_threads: usize) -> Self {
        let mut runtime = tokio::runtime::Runtime::new().unwrap();
        let executor = runtime.executor();
        let (input_command_sender, input_command_receiver) = futures::sync::mpsc::unbounded();
        let (response_sender, response_receiver) = crossbeam_channel::unbounded();

        let client = {
            let https = hyper_tls::HttpsConnector::new(number_of_dns_threads); // TODO: Number of DNS threads?
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
                        input_command_receiver
                            .take_while(|cmd| {
                                Ok(match cmd {
                                    InputCommand::Quit => {
                                        info!("Received quit command. New commands will not be received");
                                        false
                                    },
                                    _ => true,
                                })
                            }).for_each(move |cmd| {
                                let response_sender = response_sender.clone();
                                match cmd {
                                    InputCommand::Quit => unreachable!(),
                                    InputCommand::Request(req, cb) => executor.spawn(
                                        match req.http_type {
                                            RequestType::Get => client.get(req.uri.clone()),
                                        }.and_then(move |res| res.into_body().concat2())
                                        .map(move |body| {
                                            use bytes::buf::FromBuf;

                                            response_sender.send(OutputCommand::new(
                                                Response::new(
                                                    req,
                                                    Vec::from_buf(body.into_bytes()),
                                                ),
                                                cb,
                                            ))
                                        }).map(|_| {})
                                        .map_err(|e| {
                                            error!("Error on request. Info: {:?}", e);
                                        }), // TODO: Err handling?
                                    ),
                                }

                                Ok(())
                            })
                    })).unwrap();
            })
        };

        Queue {
            working_thread: Some(working_thread),
            executor,
            input_command_sender,
            response_receiver,
        }
    }

    pub fn stop(&mut self) { // TODO: Make other functions report error when queue was stopped
        self.send_input_command(InputCommand::Quit);
        mem::replace(&mut self.working_thread, None).map(|thread| {
            thread.join().unwrap();
        });
    }

    pub fn send_request<T: 'static + Fn(Response) + Send + Sync>(
        &mut self,
        request: Request,
        callback: T,
    ) {
        self.send_input_command(InputCommand::Request(request, Box::new(callback)));
    }

    fn send_input_command(&mut self, input_command: InputCommand) {
        let input_command_sender = self.input_command_sender.clone();
        self.executor.spawn(lazy(move || {
            input_command_sender
                .send(input_command)
                .map(|_| {})
                .map_err(|_| {}) // TODO: Err handling?
        }));
    }

    fn try_recv_queue(&mut self) -> Result<(), crossbeam_channel::TryRecvError> {
        let command = self.response_receiver.try_recv()?;
        (command.callback)(command.response);
        Ok(())
    }

    pub fn execute_queue_with_limit(&mut self, limit: usize, one_step_timeout: Duration) -> usize {
        let mut counter = 0;
        while counter <= limit {
            self.try_recv_queue().ok();
            thread::sleep(one_step_timeout);
            counter += 1;
        }
        counter
    }

    pub fn execute_query_with_timeout(&mut self, timeout: Duration, one_step_timeout: Duration) {
        let instant = Instant::now();

        while Instant::now().duration_since(instant) <= timeout {
            self.try_recv_queue().ok();
            thread::sleep(one_step_timeout);
        }
    }
}

mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::Mutex;

    #[test]
    fn test() {
        let mut queue = Queue::new(4);

        use std::default::Default;

        let control_variable = Arc::new(Mutex::new(false));
        let control_variable_c = Arc::clone(&control_variable);
        queue.send_request(
            RequestBuilder::default()
                .id(1)
                .http_type(RequestType::Get)
                .uri("https://docs.rs/".parse().unwrap())
                .build()
                .unwrap(),
            move |req| {
                *control_variable_c.lock().unwrap() = true;
                assert!(String::from_utf8_lossy(&req.body[..]).contains("docs.rs"));
            },
        );

        assert_eq!(*control_variable.lock().unwrap(), false);

        queue.execute_query_with_timeout(Duration::from_secs(5), Duration::from_millis(100));

        assert_eq!(*control_variable.lock().unwrap(), true);
    }
}
