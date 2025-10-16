//! Extension types for wasip3::http

pub mod body_writer;

use bytes::Bytes;
use helpers::{fields_to_header_map, get_content_length, to_internal_error_code};
use http_body::SizeHint;
use hyperium as http;
use std::{
    pin::Pin,
    task::{self, Poll},
};
use wasip3::{
    http::types::{self, ErrorCode},
    wit_bindgen::{self, StreamResult},
    wit_future,
};

pub use wasip3;

const READ_FRAME_SIZE: usize = 16 * 1024;

pub type IncomingRequestBody = IncomingBody<types::Request>;
pub type IncomingResponseBody = IncomingBody<types::Response>;

pub struct RequestOptionsExtension(pub types::RequestOptions);

impl Clone for RequestOptionsExtension {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

pub trait IncomingMessage: Unpin {
    fn get_headers(&self) -> types::Headers;

    fn consume_body(
        self,
        res: wit_bindgen::FutureReader<Result<(), ErrorCode>>,
    ) -> (
        wit_bindgen::StreamReader<u8>,
        wit_bindgen::FutureReader<Result<Option<types::Trailers>, ErrorCode>>,
    );
}

impl IncomingMessage for types::Request {
    fn get_headers(&self) -> types::Headers {
        self.get_headers()
    }

    fn consume_body(
        self,
        res: wit_bindgen::FutureReader<Result<(), ErrorCode>>,
    ) -> (
        wit_bindgen::StreamReader<u8>,
        wit_bindgen::FutureReader<Result<Option<types::Trailers>, ErrorCode>>,
    ) {
        Self::consume_body(self, res)
    }
}

impl IncomingMessage for types::Response {
    fn get_headers(&self) -> types::Headers {
        self.get_headers()
    }

    fn consume_body(
        self,
        res: wit_bindgen::FutureReader<Result<(), ErrorCode>>,
    ) -> (
        wit_bindgen::StreamReader<u8>,
        wit_bindgen::FutureReader<Result<Option<types::Trailers>, ErrorCode>>,
    ) {
        Self::consume_body(self, res)
    }
}

/// A stream of Bytes, used when receiving bodies from the network.
pub struct IncomingBody<T> {
    state: StartedState<T>,
    content_length: Option<u64>,
}

enum StartedState<T> {
    Unstarted(T),
    Started {
        #[allow(dead_code)]
        result: wit_bindgen::FutureWriter<Result<(), ErrorCode>>,
        state: IncomingState,
    },
    Empty,
}

impl<T: IncomingMessage> IncomingBody<T> {
    pub fn new(msg: T) -> Result<Self, ErrorCode> {
        let content_length = get_content_length(msg.get_headers())?;
        Ok(Self {
            state: StartedState::Unstarted(msg),
            content_length,
        })
    }

    pub fn take_unstarted(&mut self) -> Option<T> {
        match self.state {
            StartedState::Unstarted(_) => {
                let StartedState::Unstarted(msg) =
                    std::mem::replace(&mut self.state, StartedState::Empty)
                else {
                    unreachable!();
                };
                Some(msg)
            }
            _ => None,
        }
    }

    fn ensure_started(&mut self) -> Result<&mut IncomingState, ErrorCode> {
        if let StartedState::Unstarted(_) = self.state {
            let msg = self.take_unstarted().unwrap();
            let (result, reader) = wit_future::new(|| Ok(()));
            let (stream, trailers) = msg.consume_body(reader);
            self.state = StartedState::Started {
                result,
                state: IncomingState::Ready { stream, trailers },
            };
        };
        match &mut self.state {
            StartedState::Started { state, .. } => Ok(state),
            StartedState::Unstarted(_) => unreachable!(),
            StartedState::Empty => Err(to_internal_error_code(
                "cannot use IncomingBody after call to take_unstarted",
            )),
        }
    }
}

enum IncomingState {
    Ready {
        stream: wit_bindgen::StreamReader<u8>,
        trailers: wit_bindgen::FutureReader<Result<Option<types::Trailers>, ErrorCode>>,
    },
    Reading(Pin<Box<dyn std::future::Future<Output = ReadResult> + 'static + Send>>),
    Done,
}

enum ReadResult {
    Trailers(Result<Option<types::Trailers>, ErrorCode>),
    BodyChunk {
        chunk: Vec<u8>,
        stream: wit_bindgen::StreamReader<u8>,
        trailers: wit_bindgen::FutureReader<Result<Option<types::Trailers>, ErrorCode>>,
    },
}

impl<T: IncomingMessage> http_body::Body for IncomingBody<T> {
    type Data = Bytes;
    type Error = ErrorCode;

    fn poll_frame(
        mut self: Pin<&mut Self>,
        cx: &mut task::Context<'_>,
    ) -> Poll<Option<Result<http_body::Frame<Self::Data>, Self::Error>>> {
        let state = self.ensure_started()?;
        loop {
            match state {
                IncomingState::Ready { .. } => {
                    let IncomingState::Ready {
                        mut stream,
                        trailers,
                    } = std::mem::replace(state, IncomingState::Done)
                    else {
                        unreachable!();
                    };
                    *state = IncomingState::Reading(Box::pin(async move {
                        let (result, chunk) =
                            stream.read(Vec::with_capacity(READ_FRAME_SIZE)).await;
                        match result {
                            StreamResult::Complete(_n) => ReadResult::BodyChunk {
                                chunk,
                                stream,
                                trailers,
                            },
                            StreamResult::Cancelled => unreachable!(),
                            StreamResult::Dropped => ReadResult::Trailers(trailers.await),
                        }
                    }));
                }
                IncomingState::Reading(future) => {
                    match std::task::ready!(future.as_mut().poll(cx)) {
                        ReadResult::BodyChunk {
                            chunk,
                            stream,
                            trailers,
                        } => {
                            *state = IncomingState::Ready { stream, trailers };
                            break Poll::Ready(Some(Ok(http_body::Frame::data(chunk.into()))));
                        }
                        ReadResult::Trailers(trailers) => {
                            *state = IncomingState::Done;
                            match trailers {
                                Ok(Some(fields)) => {
                                    let trailers = fields_to_header_map(fields)?;
                                    break Poll::Ready(Some(Ok(http_body::Frame::trailers(
                                        trailers,
                                    ))));
                                }
                                Ok(None) => {}
                                Err(e) => {
                                    break Poll::Ready(Some(Err(e)));
                                }
                            }
                        }
                    }
                }
                IncomingState::Done => break Poll::Ready(None),
            }
        }
    }

    fn is_end_stream(&self) -> bool {
        matches!(
            self.state,
            StartedState::Started {
                state: IncomingState::Done,
                ..
            }
        )
    }

    fn size_hint(&self) -> SizeHint {
        let Some(n) = self.content_length else {
            return SizeHint::default();
        };
        let mut size_hint = SizeHint::new();
        size_hint.set_lower(0);
        size_hint.set_upper(n);
        size_hint
    }
}

pub mod helpers {
    use super::*;

    pub fn get_content_length(headers: types::Headers) -> Result<Option<u64>, ErrorCode> {
        let values = headers.get(http::header::CONTENT_LENGTH.as_str());
        if values.len() > 1 {
            return Err(to_internal_error_code("multiple content-length values"));
        }
        let Some(value_bytes) = values.into_iter().next() else {
            return Ok(None);
        };
        let value_str = std::str::from_utf8(&value_bytes).map_err(to_internal_error_code)?;
        let value_i64: i64 = value_str.parse().map_err(to_internal_error_code)?;
        let value = value_i64.try_into().map_err(to_internal_error_code)?;
        Ok(Some(value))
    }

    pub fn fields_to_header_map(headers: types::Headers) -> Result<http::HeaderMap, ErrorCode> {
        headers
            .copy_all()
            .into_iter()
            .try_fold(http::HeaderMap::new(), |mut map, (k, v)| {
                let v = http::HeaderValue::from_bytes(&v).map_err(to_internal_error_code)?;
                let k: http::HeaderName = k.parse().map_err(to_internal_error_code)?;
                map.append(k, v);
                Ok(map)
            })
    }

    pub fn scheme_from_wasi(scheme: types::Scheme) -> Result<http::uri::Scheme, ErrorCode> {
        match scheme {
            types::Scheme::Http => Ok(http::uri::Scheme::HTTP),
            types::Scheme::Https => Ok(http::uri::Scheme::HTTPS),
            types::Scheme::Other(s) => s
                .parse()
                .map_err(|_| types::ErrorCode::HttpRequestUriInvalid),
        }
    }

    pub fn scheme_to_wasi(scheme: &http::uri::Scheme) -> types::Scheme {
        match scheme {
            s if s == &http::uri::Scheme::HTTP => types::Scheme::Http,
            s if s == &http::uri::Scheme::HTTPS => types::Scheme::Https,
            other => types::Scheme::Other(other.to_string()),
        }
    }

    pub fn method_from_wasi(method: types::Method) -> Result<http::Method, ErrorCode> {
        match method {
            types::Method::Get => Ok(http::Method::GET),
            types::Method::Post => Ok(http::Method::POST),
            types::Method::Put => Ok(http::Method::PUT),
            types::Method::Delete => Ok(http::Method::DELETE),
            types::Method::Patch => Ok(http::Method::PATCH),
            types::Method::Head => Ok(http::Method::HEAD),
            types::Method::Options => Ok(http::Method::OPTIONS),
            types::Method::Connect => Ok(http::Method::CONNECT),
            types::Method::Trace => Ok(http::Method::TRACE),
            types::Method::Other(o) => http::Method::from_bytes(o.as_bytes())
                .map_err(|_| types::ErrorCode::HttpRequestMethodInvalid),
        }
    }

    pub fn method_to_wasi(method: &http::Method) -> types::Method {
        match method {
            &http::Method::GET => types::Method::Get,
            &http::Method::POST => types::Method::Post,
            &http::Method::PUT => types::Method::Put,
            &http::Method::DELETE => types::Method::Delete,
            &http::Method::PATCH => types::Method::Patch,
            &http::Method::HEAD => types::Method::Head,
            &http::Method::OPTIONS => types::Method::Options,
            &http::Method::CONNECT => types::Method::Connect,
            &http::Method::TRACE => types::Method::Trace,
            other => types::Method::Other(other.to_string()),
        }
    }

    pub fn header_map_to_wasi(map: &http::HeaderMap) -> Result<types::Fields, ErrorCode> {
        types::Fields::from_list(
            &map.iter()
                .map(|(k, v)| (k.to_string(), v.as_ref().to_vec()))
                .collect::<Vec<_>>(),
        )
        .map_err(to_internal_error_code)
    }

    pub fn header_map_to_field_entries(
        map: http::HeaderMap,
    ) -> impl Iterator<Item = (String, Vec<u8>)> {
        // https://docs.rs/http/1.3.1/http/header/struct.HeaderMap.html#method.into_iter-2
        // For each yielded item that has None provided for the HeaderName, then
        // the associated header name is the same as that of the previously
        // yielded item. The first yielded item will have HeaderName set.
        let mut last_name = None;
        map.into_iter().map(move |(name, value)| {
            if name.is_some() {
                last_name = name;
            }
            let name = last_name
                .as_ref()
                .expect("HeaderMap::into_iter always returns Some(name) before None");
            let value = bytes::Bytes::from_owner(value).to_vec();
            (name.as_str().into(), value)
        })
    }

    pub fn header_map_to_fields(map: http::HeaderMap) -> Result<types::Fields, types::HeaderError> {
        let entries = Vec::from_iter(header_map_to_field_entries(map));
        types::Fields::from_list(&entries)
    }

    pub fn to_internal_error_code(e: impl ::std::fmt::Display) -> ErrorCode {
        ErrorCode::InternalError(Some(e.to_string()))
    }
}
