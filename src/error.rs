use std::error::Error;
use std::fmt::Display;

#[derive(Debug)]
pub enum OmegalulError {
    ReqwestError(reqwest::Error),
    JsonError(json::Error),
    IdError,
    ServersError,
}

impl Display for OmegalulError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OmegalulError::ReqwestError(err) => {
                write!(f, "Something went wrong requesting data from omegle: {err}")
            }
            OmegalulError::JsonError(err) => write!(
                f,
                "Something went wrong with parsing the json response: {err}"
            ),
            OmegalulError::IdError => write!(f, "Could not get client I.D"),
            OmegalulError::ServersError => write!(f, "Could not get list of servers"),
        }
    }
}
impl Error for OmegalulError {
    fn cause(&self) -> Option<&dyn Error> {
        match self {
            OmegalulError::ReqwestError(err) => Some(err),
            OmegalulError::JsonError(err) => Some(err),
            _ => None,
        }
    }
}

impl From<reqwest::Error> for OmegalulError {
    fn from(err: reqwest::Error) -> Self {
        Self::ReqwestError(err)
    }
}

impl From<json::Error> for OmegalulError {
    fn from(err: json::Error) -> Self {
        Self::JsonError(err)
    }
}
