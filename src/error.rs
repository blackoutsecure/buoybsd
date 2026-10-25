// SPDX-License-Identifier: Apache-2.0

use std::{io, sync::Arc};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Clone, Debug, thiserror::Error)]
pub enum Error {
    #[error("{0}")]
    Message(String),
    #[error("{context}: {source}")]
    Io {
        context: String,
        source: Arc<io::Error>,
    },
    #[error("{context}: {source}")]
    Context { context: String, source: Box<Error> },
    #[error("context canceled")]
    Cancelled,
    #[error("{0}\n{1}")]
    Joined(Box<Error>, Box<Error>),
}

impl Error {
    pub fn message(message: impl Into<String>) -> Self {
        Self::Message(message.into())
    }

    pub fn io(context: impl Into<String>, source: io::Error) -> Self {
        Self::Io {
            context: context.into(),
            source: Arc::new(source),
        }
    }

    pub fn context(self, context: impl AsRef<str>) -> Self {
        Self::Context {
            context: context.as_ref().into(),
            source: Box::new(self),
        }
    }

    pub fn is_cancelled(&self) -> bool {
        match self {
            Self::Cancelled => true,
            Self::Joined(a, b) => a.is_cancelled() || b.is_cancelled(),
            Self::Context { source, .. } => source.is_cancelled(),
            _ => false,
        }
    }
}

pub fn join(result: &mut Result<()>, next: Result<()>) {
    if let Err(next) = next {
        *result = Err(match std::mem::replace(result, Ok(())) {
            Ok(()) => next,
            Err(previous) => Error::Joined(Box::new(previous), Box::new(next)),
        });
    }
}
