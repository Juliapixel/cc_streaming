//! this whole thing is kinda ass!

use std::process::Stdio;

use serde::Deserialize;
use url::Url;

#[derive(Debug, thiserror::Error)]
pub enum YtDlpError {
    #[error(transparent)]
    SerdeError(#[from] serde_json::Error),
    #[error(transparent)]
    IoError(#[from] std::io::Error),
}

impl actix_web::ResponseError for YtDlpError {}

// tons of allocations and stuff but i cba to actually make it efficient rn

#[derive(Debug, Deserialize)]
pub struct YtDlpInfo {
    pub id: Option<String>,
    pub display_id: Option<String>,
    pub url: String,
    pub title: Option<String>,
    pub description: Option<String>,
    pub uploader: Option<String>,
    pub uploader_id: Option<String>,
    pub timestamp: f64,
    pub formats: Option<Vec<Format>>,
    pub width: Option<i64>,
    pub height: Option<i64>,
    pub video_ext: String,
    pub audio_ext: String,
    pub vcodec: Option<String>,
    pub acodec: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct Format {
    pub format_id: String,
    pub url: String,
    pub width: Option<i64>,
    pub height: Option<i64>,
    pub fps: Option<f32>,
    pub vcodec: String,
    pub acodec: String,
    pub video_ext: String,
    pub audio_ext: String,
}

enum FormatKind {
    AudioOnly,
    VideoOnly,
    Both,
    Neither,
}

impl FormatKind {
    pub fn from_format(format: &Format) -> Self {
        let video_is_none = format.vcodec == "none" || format.video_ext == "none";
        let audio_is_none =
            format.acodec == "none" || (video_is_none && format.audio_ext == "none");
        match (video_is_none, audio_is_none) {
            (true, true) => Self::Both,
            (true, false) => Self::AudioOnly,
            (false, true) => Self::VideoOnly,
            (false, false) => Self::Neither,
        }
    }

    pub fn from_info(info: &YtDlpInfo) -> Self {
        let video_is_none =
            info.vcodec.as_ref().is_none_or(|v| v == "none") || info.video_ext == "none";
        let audio_is_none = info.acodec.as_ref().is_none_or(|a| a == "none")
            || (video_is_none && info.audio_ext == "none");
        match (video_is_none, audio_is_none) {
            (true, true) => Self::Both,
            (true, false) => Self::AudioOnly,
            (false, true) => Self::VideoOnly,
            (false, false) => Self::Neither,
        }
    }
}

impl YtDlpInfo {
    pub async fn new(url: &Url) -> Result<Self, YtDlpError> {
        let out = tokio::process::Command::new("/usr/bin/env")
            .arg("yt-dlp")
            .arg("-j")
            .arg(url.as_str())
            .stdout(Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .unwrap()
            .wait_with_output()
            .await?;
        serde_json::from_slice::<Self>(&out.stdout).map_err(Into::into)
    }

    pub fn best_video_match(&self, width: u32, height: u32) -> Option<&str> {
        match &self.formats {
            Some(f) => {
                let mut closest = 0;
                let mut closest_height = i64::MAX;
                for (i, format) in f.iter().enumerate() {
                    let format_height = format.height.unwrap_or(i64::MAX);
                    if format_height < closest_height && format_height > height as i64 {
                        closest = i;
                        closest_height = format_height
                    }
                }
                if closest_height == i64::MAX {
                    None
                } else {
                    Some(&f[closest].url)
                }
            }
            None => match FormatKind::from_info(self) {
                FormatKind::Both | FormatKind::VideoOnly => Some(&self.url),
                _ => None,
            },
        }
    }

    pub fn formats(&self) -> impl IntoIterator<Item = &Format> {
        self.formats.as_ref().map(|f| f.iter()).unwrap_or_default()
    }
}
