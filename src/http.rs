use anyhow::Result;
use serde::de::DeserializeOwned;
use serde::Serialize;

#[derive(Clone)]
pub struct PlainHttp {
    inner: wreq::Client,
}

impl PlainHttp {
    pub fn new() -> Self {
        Self {
            inner: wreq::Client::builder().build().expect("build wreq client"),
        }
    }

    pub fn post(&self, url: &str) -> RequestBuilder {
        RequestBuilder {
            inner: self.inner.post(url),
        }
    }

    pub fn get(&self, url: &str) -> RequestBuilder {
        RequestBuilder {
            inner: self.inner.get(url),
        }
    }
}

impl Default for PlainHttp {
    fn default() -> Self {
        Self::new()
    }
}

pub struct RequestBuilder {
    inner: wreq::RequestBuilder,
}

impl RequestBuilder {
    pub fn json<T: Serialize>(self, body: &T) -> Self {
        Self {
            inner: self.inner.json(body),
        }
    }

    pub fn header(self, name: &'static str, value: &str) -> Self {
        Self {
            inner: self.inner.header(name, value),
        }
    }

    pub async fn send(self) -> Result<Response> {
        Ok(Response {
            inner: self.inner.send().await?,
        })
    }
}

pub struct Response {
    inner: wreq::Response,
}

impl Response {
    pub async fn json<T: DeserializeOwned>(self) -> Result<T> {
        Ok(self.inner.json::<T>().await?)
    }
}
