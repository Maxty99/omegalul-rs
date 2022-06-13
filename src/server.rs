use crate::error::OmegalulError;
use crate::id::*;
use async_stream::try_stream;
use futures_core::Stream;
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

    fn parse_events(json: Vec<JsonValue>) -> Vec<ChatEvent> {
        let mut events_list: Vec<ChatEvent> = vec![];
        for event in json {
            let array = as_array(&event);
            let event_name = event[0].as_str().unwrap();

            match event_name {
                "gotMessage" => {
                    events_list.push(ChatEvent::Message(array[1].as_str().unwrap().to_owned()))
                }
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
                "strangerDisconnected" => events_list.push(ChatEvent::StrangerDisconnected),
                _ => {}
            }
        }
        events_list
    }

    pub async fn start_chat(&self) -> Result<Chat, OmegalulError> {
        let random_id = generate_random_id();
        let omegle_url = format!("{}.omegle.com", self.name);

        let interests: Vec<String> = self
            .interests
            .iter()
            .map(|str| format!("\"{str}\""))
            .collect();

        let interests_str = interests.join(",");

        let response = self
            .client
            .post(format!(
                "https://{}/start?caps=recaptcha2,t&firstevents=1&spid=&randid={}&lang=en&topics=[{}]",
                omegle_url, random_id, interests_str
            ))
            .send()
            .await?;

        let response_text = response.text().await?;
        let parsed_json = json::parse(&response_text)?;
        let response_array = as_array(&parsed_json["events"]);
        let events_list = Server::parse_events(response_array);
        let id_json_value = parsed_json["clientID"].clone();
        match id_json_value {
            JsonValue::String(id_string) => Ok(Chat::new(id_string, self.clone(), events_list)),
            _ => Err(OmegalulError::IdError),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Chat {
    pub client_id: String,
    server: Server,
    initial_events: Vec<ChatEvent>,
}

impl Chat {
    pub fn new(client_id: String, server: Server, initial_events: Vec<ChatEvent>) -> Self {
        return Self {
            client_id,
            server,
            initial_events,
        };
    }

    pub async fn fetch_event(&self) -> Result<Vec<ChatEvent>, OmegalulError> {
        let server = &self.server;
        let omegle_url = format!("{}.omegle.com", server.name);
        let pair = [("id", self.client_id.clone())];

        let response = server
            .client
            .post(format!("https://{}/events", omegle_url))
            .form(&pair)
            .send()
            .await?;

        let response_text = response.text().await?;
        let parsed_json = json::parse(&response_text)?;

        let response_array = as_array(&parsed_json);

        Ok(Server::parse_events(response_array))
    }

    pub fn get_event_stream(
        &self,
    ) -> impl Stream<Item = Result<Vec<ChatEvent>, OmegalulError>> + '_ {
        try_stream! {
            let mut sent_initial = false;
            let mut break_on_next = false;
            loop {
                if !sent_initial {
                    sent_initial = true;
                    yield self.initial_events.clone();
                }
                if break_on_next{
                    break;
                }
                let events = self.fetch_event().await?;
                if events.contains(&ChatEvent::StrangerDisconnected){
                    break_on_next = true;
                }
                yield events;
            }
        }
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

#[derive(Debug, Clone, PartialEq)]
pub enum ChatEvent {
    Message(String),
    CommonLikes(Vec<String>),
    Connected,
    StrangerDisconnected,
    Typing,
    StoppedTyping,
    Waiting,
}
