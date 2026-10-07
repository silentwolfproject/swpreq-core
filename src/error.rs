use std::ffi::CString;
use std::os::raw::c_char;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum SwpreqError {
    #[error("network error: {0}")]
    Network(String),

    #[error("timeout: {0}")]
    Timeout(String),

    #[error("invalid url: {0}")]
    InvalidUrl(String),

    #[error("tls error: {0}")]
    Tls(String),

    #[error("proxy error: {0}")]
    Proxy(String),

    #[error("decode error: {0}")]
    Decode(String),

    #[error("encode error: {0}")]
    Encode(String),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("invalid argument: {0}")]
    InvalidArgument(String),

    #[error("not found: {0}")]
    NotFound(String),

    #[error("internal error: {0}")]
    Internal(String),

}

pub type Result<T> = std::result::Result<T, SwpreqError>;

impl SwpreqError {
    pub fn code(&self) -> i32 {
        match self {
            Self::Network(_) => 1,
            Self::Timeout(_) => 2,
            Self::InvalidUrl(_) => 3,
            Self::Tls(_) => 4,
            Self::Proxy(_) => 5,
            Self::Decode(_) => 6,
            Self::Encode(_) => 7,
            Self::Io(_) => 8,
            Self::InvalidArgument(_) => 9,
            Self::NotFound(_) => 10,
            Self::Internal(_) => 99,
        }
    }
}

pub fn to_c_string(s: String) -> *mut c_char {
    CString::new(s)
        .unwrap_or_else(|_| CString::new("").unwrap())
        .into_raw()
}

pub fn from_c_string(ptr: *const c_char) -> Option<String> {
    if ptr.is_null() {
        return None;
    }
    unsafe { std::ffi::CStr::from_ptr(ptr).to_str().ok().map(|s| s.to_string()) }
}