use crate::error::{Result, SwpreqError};
use reqwest::multipart::{Form, Part};
use std::path::Path;

pub struct MultipartBuilder {
    form: Form,
}

impl MultipartBuilder {
    pub fn new() -> Self {
        Self {
            form: Form::new(),
        }
    }

    pub fn text(mut self, name: &str, value: &str) -> Self {
        self.form = self.form.text(name.to_string(), value.to_string());
        self
    }

    pub fn bytes(mut self, name: &str, data: Vec<u8>, filename: &str, mime: &str) -> Result<Self> {
        let part = Part::bytes(data)
            .file_name(filename.to_string())
            .mime_str(mime)
            .map_err(|e| SwpreqError::Encode(e.to_string()))?;
        self.form = self.form.part(name.to_string(), part);
        Ok(self)
    }

    pub async fn file(mut self, name: &str, path: &str) -> Result<Self> {
        let data = tokio::fs::read(path)
            .await
            .map_err(|e| SwpreqError::Io(e))?;

        let filename = Path::new(path)
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "file".to_string());

        let mime = mime_guess::from_path(path)
            .first_or_octet_stream()
            .to_string();

        let part = Part::bytes(data)
            .file_name(filename)
            .mime_str(&mime)
            .map_err(|e| SwpreqError::Encode(e.to_string()))?;

        self.form = self.form.part(name.to_string(), part);
        Ok(self)
    }

    pub fn build(self) -> Form {
        self.form
    }
}

impl Default for MultipartBuilder {
    fn default() -> Self {
        Self::new()
    }
}