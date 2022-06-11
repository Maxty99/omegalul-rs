use crate::id::*;
use json::JsonValue;
use reqwest::Client;

use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};

pub async fn get_random_server() -> Option<String> {
    let servers = get_servers().await;

    if let Some(servers) = servers {
        if let JsonValue::Array(array) = servers {
            return match array.choose(&mut rand::thread_rng()) {
                Some(random) => Some(random.as_str().unwrap().to_string()),
                None => None,
            };
        }
    }

    return None;
}

pub async fn get_servers() -> Option<JsonValue> {
    let client = Client::new();
    let request = client.get("https://omegle.com/status").send().await;

    return match request {
        Ok(request) => {
            Some(json::parse(request.text().await.unwrap().as_str()).unwrap()["servers"].clone())
        }
        Err(_error) => None,
    };
}

#[derive(Debug, Clone)]
pub struct Server {
    name: String,
    interests: Vec<String>,
    client: Client,
}

impl Server {
    pub fn new(name: &str, interests: Vec<String>) -> Self {
        Self {
            name: name.to_string(),
            interests: interests,
            client: Client::new(),
        }
    }

    pub async fn start_chat(&self) -> (Option<Chat>, Vec<ChatEvent>) {
        let random_id = generate_random_id();
        let omegle_url = format!("{}.omegle.com", self.name);

        let mut interests_str = "".to_owned();

        for i in 0..self.interests.len() {
            interests_str.push_str(&format!("\"{}\"", self.interests[i]));

            if i != self.interests.len() - 1 {
                interests_str.push(',');
            }
        }

        let response = self
            .client
            .post(format!(
                "https://{}/start?caps=recaptcha2,t&firstevents=1&spid=&randid={}&lang=en&topics=[{}]",
                omegle_url, random_id, interests_str
            ))
            .send()
            .await;

        let mut events_list: Vec<ChatEvent> = vec![];
        let mut chat: Option<Chat> = None;
        if let Ok(response) = response {
            if let Ok(response_text) = response.text().await {
                chat = match json::parse(&response_text) {
                    Ok(json) => Some(Chat::new(
                        json["clientID"].clone().as_str().unwrap(),
                        self.clone(),
                    )),
                    Err(_error) => None,
                };
                match json::parse(&response_text) {
                    Ok(json_response) => {
                        let response_array = as_array(&json_response);

                        for event in response_array {
                            let array = as_array(&event);
                            let event_name = event[0].as_str().unwrap();

                            match event_name {
                                "gotMessage" => events_list.push(ChatEvent::Message(
                                    array[1].as_str().unwrap().to_owned(),
                                )),
                                "connected" => events_list.push(ChatEvent::Connected),
                                "commonLikes" => events_list.push(ChatEvent::CommonLikes(
                                    as_array(&array[1])
                                        .iter()
                                        .map(|x| x.as_str().unwrap().to_owned())
                                        .collect(),
                                )),
                                "waiting" => events_list.push(ChatEvent::Waiting),
                                "typing" => events_list.push(ChatEvent::Typing),
                                "stoppedTyping" => events_list.push(ChatEvent::StoppedTyping),
                                "strangerDisconnected" => {
                                    events_list.push(ChatEvent::StrangerDisconnected)
                                }
                                _ => {}
                            }
                        }
                    }
                    Err(_err) => {}
                };
            }
        }

        return (chat, events_list);
    }
}

#[derive(Debug, Clone)]
pub struct Chat {
    pub client_id: String,
    server: Server,
}

impl Chat {
    pub fn new(client_id: &str, server: Server) -> Self {
        return Self {
            client_id: client_id.to_string(),
            server: server,
        };
    }

    pub async fn fetch_event(&self) -> Vec<ChatEvent> {
        let server = &self.server;
        let omegle_url = format!("{}.omegle.com", server.name);
        let pair = [("id", self.client_id.clone())];

        let response = server
            .client
            .post(format!("https://{}/events", omegle_url))
            .form(&pair)
            .send()
            .await;
        let mut events_list: Vec<ChatEvent> = vec![];
        if let Ok(response) = response {
            if let Ok(response_text) = response.text().await {
                match json::parse(&response_text) {
                    Ok(json_response) => {
                        let response_array = as_array(&json_response);

                        for event in response_array {
                            let array = as_array(&event);
                            let event_name = event[0].as_str().unwrap();

                            match event_name {
                                "gotMessage" => events_list.push(ChatEvent::Message(
                                    array[1].as_str().unwrap().to_owned(),
                                )),
                                "connected" => events_list.push(ChatEvent::Connected),
                                "commonLikes" => events_list.push(ChatEvent::CommonLikes(
                                    as_array(&array[1])
                                        .iter()
                                        .map(|x| x.as_str().unwrap().to_owned())
                                        .collect(),
                                )),
                                "waiting" => events_list.push(ChatEvent::Waiting),
                                "typing" => events_list.push(ChatEvent::Typing),
                                "stoppedTyping" => events_list.push(ChatEvent::StoppedTyping),
                                "strangerDisconnected" => {
                                    events_list.push(ChatEvent::StrangerDisconnected)
                                }
                                _ => {}
                            }
                        }
                    }
                    Err(_err) => {}
                };
            }
        }

        events_list
    }

    pub async fn send_message(&mut self, message: &str) {
        self.handle_server_post(
            "send",
            &[("id", self.client_id.clone()), ("msg", message.to_owned())],
        )
        .await;
    }

    pub async fn disconnect(&mut self) {
        self.handle_server_post("disconnect", &[("id", self.client_id.clone())])
            .await;
    }

    pub async fn start_typing(&mut self) {
        self.handle_server_post("typing", &[("id", self.client_id.clone())])
            .await;
    }

    pub async fn stop_typing(&mut self) {
        self.handle_server_post("stoppedtyping", &[("id", self.client_id.clone())])
            .await;
    }

    pub async fn handle_server_post<K: Serialize, V: Serialize>(
        &mut self,
        path: &str,
        pair: &[(K, V)],
    ) {
        let server = &mut self.server;
        let omegle_url = format!("{}.omegle.com", server.name);

        handle_simple_post(
            server.client.clone(),
            &format!("https://{}/{}", omegle_url, path),
            pair,
        )
        .await;
    }
}

async fn handle_simple_post<K: Serialize, V: Serialize>(
    client: Client,
    url: &str,
    pair: &[(K, V)],
) {
    let response = client.post(format!("{}", url)).form(&pair).send().await;

    if let Err(error) = response {
        println!("{:?}", error);
    }
}

fn as_array(value: &JsonValue) -> Vec<JsonValue> {
    match value {
        JsonValue::Array(array) => array.to_vec(),
        _ => vec![],
    }
}

#[derive(Debug, Clone)]
pub enum ChatEvent {
    Message(String),
    CommonLikes(Vec<String>),
    Connected,
    StrangerDisconnected,
    Typing,
    StoppedTyping,
    Waiting,
}
