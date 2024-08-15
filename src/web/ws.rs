use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct StreamVideoFrame {
    pub palette: Vec<[u8; 3]>,
    pub rows: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct StreamAudioFrame {
    pub samples: Vec<u8>,
}
