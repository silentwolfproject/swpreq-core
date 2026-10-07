use crate::error::Result;
use tokio::io::AsyncRead;

pub struct StreamBody {
    pub reader: Box<dyn AsyncRead + Send + Unpin>,
    pub length: Option<u64>,
    pub content_type: Option<String>,
}

impl StreamBody {
    pub fn from_file(path: &str) -> Result<Self> {
        let file = std::fs::File::open(path)?;
        let metadata = file.metadata()?;
        let length = metadata.len();
        let async_file = tokio::fs::File::from_std(file);

        let content_type = mime_guess::from_path(path)
            .first_or_octet_stream()
            .to_string();

        Ok(Self {
            reader: Box::new(async_file),
            length: Some(length),
            content_type: Some(content_type),
        })
    }

    pub fn from_bytes(data: Vec<u8>) -> Self {
        let length = data.len() as u64;
        let reader = std::io::Cursor::new(data);

        Self {
            reader: Box::new(reader),
            length: Some(length),
            content_type: None,
        }
    }

    pub fn from_reader<R>(reader: R, length: Option<u64>) -> Self
    where
        R: AsyncRead + Send + Unpin + 'static,
    {
        Self {
            reader: Box::new(reader),
            length,
            content_type: None,
        }
    }

    pub fn with_content_type(mut self, ct: &str) -> Self {
        self.content_type = Some(ct.to_string());
        self
    }
}